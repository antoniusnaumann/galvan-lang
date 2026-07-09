// Registers a highlight.js definition for Galvan and re-highlights all
// Galvan code blocks. mdBook loads this file after its own highlighting
// pass, so blocks tagged `galvan` are plain until this script runs.
(function () {
  if (typeof hljs === "undefined") {
    return;
  }

  hljs.registerLanguage("galvan", function (hljs) {
    var KEYWORDS = {
      keyword:
        "fn type let mut ref pub test cmd try else if for match throw " +
        "return use async auto trait where in and or xor not break " +
        "continue loop self it",
      literal: "true false none",
      built_in: "print println debug panic assert format",
    };

    var STRING = {
      className: "string",
      begin: '"',
      end: '"',
      contains: [
        { begin: "\\\\." },
        {
          className: "subst",
          begin: "\\\\\\(",
          end: "\\)",
          keywords: KEYWORDS,
        },
      ],
    };

    var CHAR = {
      className: "string",
      begin: "'(\\\\.|[^'])'",
    };

    return {
      name: "Galvan",
      keywords: KEYWORDS,
      contains: [
        hljs.C_LINE_COMMENT_MODE,
        STRING,
        CHAR,
        {
          className: "type",
          begin: "\\b[A-Z][a-zA-Z0-9]*\\b",
        },
        {
          className: "number",
          begin: "\\b\\d[\\d_]*(\\.[\\d_]+)?\\b",
        },
        {
          className: "meta",
          begin: "@[a-zA-Z_][a-zA-Z0-9_]*",
        },
      ],
    };
  });

  var highlight = hljs.highlightElement || hljs.highlightBlock;
  document
    .querySelectorAll("code.language-galvan")
    .forEach(function (block) {
      highlight.call(hljs, block);
    });
})();
