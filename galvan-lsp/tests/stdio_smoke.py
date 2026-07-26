#!/usr/bin/env python3
"""Smoke test: drive galvan-lsp over stdio through a realistic session."""
import json
import os
import subprocess
import sys
import tempfile

# Resolved relative to this file; override with GALVAN_LSP_BIN.
BINARY = os.environ.get(
    "GALVAN_LSP_BIN",
    os.path.normpath(os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "..", "..", "target", "debug", "galvan-lsp",
    )),
)

SOURCE = """fn greet(name: String) {
    greet(name)
}

type Color {
    Transparent,
    Gray(U8),
}

fn main_fn() {
    let color = Color::Transparent
    println(color)
}
"""


def message(payload: dict) -> bytes:
    body = json.dumps(payload).encode()
    return f"Content-Length: {len(body)}\r\n\r\n".encode() + body


def read_message(stream):
    headers = {}
    while True:
        line = stream.readline().decode()
        if line in ("\r\n", "\n", ""):
            break
        key, _, value = line.partition(":")
        headers[key.strip().lower()] = value.strip()
    length = int(headers["content-length"])
    return json.loads(stream.read(length))


def main():
    root = tempfile.mkdtemp(prefix="galvan_lsp_smoke_")
    src = os.path.join(root, "src")
    os.makedirs(src)
    path = os.path.join(src, "main.galvan")
    with open(path, "w") as f:
        f.write(SOURCE)
    uri = "file://" + path

    proc = subprocess.Popen(
        [BINARY], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    w = proc.stdin

    def send(payload):
        w.write(message(payload))
        w.flush()

    def request(rid, method, params):
        send({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
        while True:
            msg = read_message(proc.stdout)
            if msg.get("id") == rid:
                return msg

    def expect_notification(method):
        while True:
            msg = read_message(proc.stdout)
            if msg.get("method") == method:
                return msg

    # -- initialize ------------------------------------------------------
    reply = request(1, "initialize", {"capabilities": {}})
    caps = reply["result"]["capabilities"]
    for cap in [
        "hoverProvider", "definitionProvider", "referencesProvider",
        "completionProvider", "renameProvider", "documentSymbolProvider",
        "workspaceSymbolProvider", "inlayHintProvider", "signatureHelpProvider",
        "semanticTokensProvider", "codeActionProvider", "documentFormattingProvider",
        "typeDefinitionProvider", "documentHighlightProvider", "foldingRangeProvider",
        "selectionRangeProvider", "documentRangeFormattingProvider",
        "documentOnTypeFormattingProvider",
    ]:
        assert cap in caps, f"missing capability {cap}: {caps.keys()}"
    send({"jsonrpc": "2.0", "method": "initialized", "params": {}})

    # -- didOpen (expect publishDiagnostics notification) ----------------
    send({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": {"uri": uri, "languageId": "galvan", "version": 1, "text": SOURCE},
    }})
    note = expect_notification("textDocument/publishDiagnostics")
    assert note["params"]["diagnostics"] == [], note["params"]

    # -- completion after `Color::` --------------------------------------
    line = SOURCE.splitlines().index("    let color = Color::Transparent")
    char = len("    let color = Color::")
    reply = request(2, "textDocument/completion", {
        "textDocument": {"uri": uri},
        "position": {"line": line, "character": char},
    })
    labels = sorted(item["label"] for item in reply["result"])
    assert labels == ["Gray", "Transparent"], labels

    # -- hover at END of identifier (the end-inclusive fix) --------------
    reply = request(3, "textDocument/hover", {
        "textDocument": {"uri": uri},
        "position": {"line": 1, "character": len("    greet")},
    })
    assert "fn greet" in reply["result"]["contents"]["value"], reply["result"]

    # -- documentSymbol ---------------------------------------------------
    reply = request(4, "textDocument/documentSymbol", {"textDocument": {"uri": uri}})
    names = {s["name"]: s for s in reply["result"]}
    assert "Color" in names and "greet" in names, names.keys()
    children = [c["name"] for c in names["Color"].get("children") or []]
    assert "Transparent" in children, children

    # -- rename `greet` ----------------------------------------------------
    reply = request(5, "textDocument/rename", {
        "textDocument": {"uri": uri},
        "position": {"line": 0, "character": 4},
        "newName": "welcome",
    })
    edits = reply["result"]["changes"][uri]
    assert len(edits) == 2, edits  # declaration + recursive call

    # -- inlayHint ---------------------------------------------------------
    reply = request(6, "textDocument/inlayHint", {
        "textDocument": {"uri": uri},
        "range": {"start": {"line": 0, "character": 0}, "end": {"line": 99, "character": 0}},
    })
    hint_labels = [h["label"] for h in reply["result"]]
    assert ": Color" in hint_labels, hint_labels

    # -- signatureHelp inside `greet(name)` --------------------------------
    reply = request(7, "textDocument/signatureHelp", {
        "textDocument": {"uri": uri},
        "position": {"line": 1, "character": len("    greet(")},
    })
    signatures = reply["result"]["signatures"]
    assert signatures[0]["label"] == "fn greet(name: String)", signatures
    assert reply["result"]["activeParameter"] == 0, reply["result"]

    # -- semanticTokens/full ------------------------------------------------
    reply = request(8, "textDocument/semanticTokens/full", {
        "textDocument": {"uri": uri},
    })
    data = reply["result"]["data"]
    assert data and len(data) % 5 == 0, f"malformed token data: {data[:10]}"

    # -- codeAction on the unannotated `let color` --------------------------
    reply = request(9, "textDocument/codeAction", {
        "textDocument": {"uri": uri},
        "range": {"start": {"line": line, "character": 0},
                  "end": {"line": line, "character": 0}},
        "context": {"diagnostics": []},
    })
    titles = [action["title"] for action in reply["result"]]
    assert "Add type annotation `: Color` to `color`" in titles, titles

    # -- formatting (the source is already well formatted) -------------------
    reply = request(10, "textDocument/formatting", {
        "textDocument": {"uri": uri},
        "options": {"tabSize": 4, "insertSpaces": True},
    })
    assert reply["result"] == [], reply["result"]

    # -- documentHighlight on a use of `color` -------------------------------
    println_line = SOURCE.splitlines().index("    println(color)")
    reply = request(11, "textDocument/documentHighlight", {
        "textDocument": {"uri": uri},
        "position": {"line": println_line, "character": len("    println(c")},
    })
    assert len(reply["result"]) == 2, reply["result"]  # binding + use

    # -- foldingRange ---------------------------------------------------------
    reply = request(12, "textDocument/foldingRange", {"textDocument": {"uri": uri}})
    kinds = {r.get("kind") for r in reply["result"]}
    assert "region" in kinds, reply["result"]

    # -- typeDefinition on `color` jumps to `type Color` ----------------------
    reply = request(13, "textDocument/typeDefinition", {
        "textDocument": {"uri": uri},
        "position": {"line": println_line, "character": len("    println(c")},
    })
    type_line = SOURCE.splitlines().index("type Color {")
    assert reply["result"]["range"]["start"]["line"] == type_line, reply["result"]

    # -- selectionRange expands outward ---------------------------------------
    reply = request(14, "textDocument/selectionRange", {
        "textDocument": {"uri": uri},
        "positions": [{"line": println_line, "character": len("    println(c")}],
    })
    assert len(reply["result"]) == 1 and "parent" in reply["result"][0], reply["result"]

    # -- rangeFormatting / onTypeFormatting (source is well formatted) --------
    reply = request(15, "textDocument/rangeFormatting", {
        "textDocument": {"uri": uri},
        "range": {"start": {"line": 0, "character": 0}, "end": {"line": 99, "character": 0}},
        "options": {"tabSize": 4, "insertSpaces": True},
    })
    assert reply["result"] == [], reply["result"]
    reply = request(16, "textDocument/onTypeFormatting", {
        "textDocument": {"uri": uri},
        "position": {"line": 2, "character": 1},
        "ch": "}",
        "options": {"tabSize": 4, "insertSpaces": True},
    })
    assert reply["result"] == [], reply["result"]

    # -- foreign keyword: didChange breaks the file, quickfix repairs it ------
    broken = SOURCE.replace("fn greet", "func greet")
    send({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": 2},
        "contentChanges": [{"text": broken}],
    }})
    note = expect_notification("textDocument/publishDiagnostics")
    foreign = [d for d in note["params"]["diagnostics"]
               if d.get("code") == "foreign_keyword"]
    assert foreign, note["params"]
    assert "Galvan uses `fn`" in foreign[0]["message"], foreign[0]

    reply = request(17, "textDocument/codeAction", {
        "textDocument": {"uri": uri},
        "range": foreign[0]["range"],
        "context": {"diagnostics": foreign},
    })
    titles = [action["title"] for action in reply["result"]]
    assert "Replace `func` with `fn`" in titles, titles

    # -- apply the quickfix through an *incremental* didChange ---------------
    fix = next(action for action in reply["result"]
               if action["title"] == "Replace `func` with `fn`")
    edit = fix["edit"]["changes"][uri][0]
    send({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": 3},
        "contentChanges": [{"range": edit["range"], "text": edit["newText"]}],
    }})
    note = expect_notification("textDocument/publishDiagnostics")
    assert note["params"]["diagnostics"] == [], note["params"]

    # -- didClose clears diagnostics ---------------------------------------
    send({"jsonrpc": "2.0", "method": "textDocument/didClose", "params": {
        "textDocument": {"uri": uri},
    }})
    note = expect_notification("textDocument/publishDiagnostics")
    assert note["params"]["diagnostics"] == [], note["params"]

    request(18, "shutdown", None)
    send({"jsonrpc": "2.0", "method": "exit"})
    w.close()
    try:
        proc.wait(timeout=5)
        print("server exited cleanly on exit notification")
    except subprocess.TimeoutExpired:
        proc.kill()
        print("NOTE: server did not exit within 5s of the exit notification")
    print("SMOKE TEST PASSED: capabilities, diagnostics, ::-completion, "
          "end-of-ident hover, symbols, rename, inlay hints, signature help, "
          "semantic tokens, code actions, formatting, document highlight, "
          "folding, type definition, selection range, range/on-type formatting, "
          "foreign-keyword quickfix, incremental sync, close-clears-diags")


if __name__ == "__main__":
    sys.exit(main())
