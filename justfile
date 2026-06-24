# === Developer environments === #

# Run all checks (linter + formatting)
lint: fmt clippy check-unused-deps

# Correction of code style according to
fmt:
    @echo "🍰 Adding styles.."
    @cargo +nightly fmt --all

# Check and automatically apply simple clippy tips
clippy:
    @echo "✨ Correcting inconsistencies and organizing code.."
    @cargo clippy \
        --fix \
        --workspace \
        --all-targets \
        --all-features \
        --allow-dirty \
        --allow-staged \
        -- \
        -D warnings

# Checking for unused dependencies in Cargo.toml
check-unused-deps:
    @echo "🔍 Checking for unused dependencies in Cargo.toml..."
    @cargo machete

# Checking for "forgotten" and unused code
check-dead-code:
    @cargo check --profile test -- -D dead-code -D unused-imports
