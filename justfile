set quiet := true

c_step  := '\033[1;34m'
c_done  := '\033[1;32m'
c_reset := '\033[0m'

# === Developer environments === #

# Run all checks (linter + formatting)
lint: fmt clippy unused
    @printf "{{c_done}}=> [4/4] ✔ All checks passed successfully!{{c_reset}}\n"

# Correction of code style
fmt:
    @printf "{{c_step}}=> [1/4] 🍰 Formatting code..{{c_reset}}\n"
    cargo +nightly fmt --all

# Check and automatically apply simple clippy tips
clippy:
    @printf "{{c_step}}=> [2/4] ✨ Running clippy..{{c_reset}}\n"
    cargo clippy \
        --fix \
        --workspace \
        --all-targets \
        --all-features \
        --allow-dirty \
        --allow-staged \
        -- \
        -D warnings

# Checking for unused dependencies in Cargo.toml
unused:
    @printf "{{c_step}}=> [3/4] 🔍 Checking unused deps..{{c_reset}}\n"
    cargo machete

# Checking dependencies for security vulnerabilities
audit:
    @printf "{{c_step}}=> 🛡️ Running security audit..{{c_reset}}\n"
    cargo audit

# Check for outdated dependencies
outdated:
    @printf "{{c_step}}=> 📦 Checking for outdated crates..{{c_reset}}\n"
    cargo outdated --workspace

# Clean build artifacts and temporary files
clean:
    @printf "{{c_step}}=> 🧹 Cleaning project..{{c_reset}}\n"
    cargo clean
    @printf "{{c_done}}✔ Workspace is clean!{{c_reset}}\n"


# Bumping version of the project (Usage: just bump minor или just bump major)
bump LEVEL="patch":
    @printf "{{c_step}}=> 📦 Bumping version to {{LEVEL}} level..{{c_reset}}\n"
    cargo release {{LEVEL}} --workspace --no-publish --execute