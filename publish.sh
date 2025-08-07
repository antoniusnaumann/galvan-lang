#!/bin/bash
set -e

# Parse command line arguments
DRY_RUN=""
if [ "$1" = "--dry-run" ]; then
    DRY_RUN="--dry-run"
    echo "Running in dry-run mode..."
fi

# Function to publish a crate with error handling
publish_crate() {
    local crate_name="$1"
    local crate_path="$2"
    
    echo "Publishing $crate_name..."
    
    if [ "$crate_path" != "." ]; then
        cd "$crate_path"
    fi
    
    # Run cargo publish and capture output
    if output=$(cargo publish $DRY_RUN --allow-dirty 2>&1); then
        if [ -n "$DRY_RUN" ]; then
            echo "✓ $crate_name: dry-run successful"
        else
            echo "✓ $crate_name: published successfully"
        fi
    else
        # Check if the error is due to crate already being published
        if echo "$output" | grep -q "already exists on crates.io index"; then
            echo "ℹ $crate_name: already published, skipping"
        else
            echo "✗ $crate_name: failed to publish"
            echo "$output"
            if [ "$crate_path" != "." ]; then
                cd ..
            fi
            exit 1
        fi
    fi
    
    if [ "$crate_path" != "." ]; then
        cd ..
    fi
}

echo "Publishing Galvan crates in dependency order..."

# tree-sitter-galvan (no galvan dependencies)
publish_crate "tree-sitter-galvan" "tree-sitter-galvan"

# galvan-files (no galvan dependencies)
publish_crate "galvan-files" "galvan-files"

# galvan-ast-macro (no galvan dependencies)
publish_crate "galvan-ast-macro" "galvan-ast-macro"

# galvan-test-macro (no galvan dependencies)
publish_crate "galvan-test-macro" "galvan-test-macro"

# galvan-ast (depends on galvan-files, galvan-ast-macro)
publish_crate "galvan-ast" "galvan-ast"

# galvan-parse (depends on galvan-files, tree-sitter-galvan)
publish_crate "galvan-parse" "galvan-parse"

# galvan-resolver (depends on galvan-ast)
publish_crate "galvan-resolver" "galvan-resolver"

# galvan-into-ast (depends on galvan-files, galvan-ast, galvan-parse)
publish_crate "galvan-into-ast" "galvan-into-ast"

# galvan-transpiler (depends on galvan-files, galvan-ast, galvan-resolver, galvan-into-ast)
publish_crate "galvan-transpiler" "galvan-transpiler"

# galvan (main crate, depends on galvan-transpiler)
publish_crate "galvan" "."

echo "✓ All crates processed successfully!"
