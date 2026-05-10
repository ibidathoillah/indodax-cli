# TODO

## High Priority
- [ ] Implement `ws orders` private order streaming (currently a stub)
- [ ] Add CI workflow to run `cargo test` on push/PR
- [ ] Implement proper pending/limit order state for paper trading CLI (orders currently fill instantly)
- [ ] Replace hardcoded WebSocket JWT token with dynamic token generation via API

## Medium Priority
- [ ] Add custom domain for GitHub Pages site
- [ ] Commit `Cargo.lock` to repo (recommended for binary applications)

## Low Priority
- [ ] Improve error messages for test panics (use descriptive assertions instead of `panic!`)
- [ ] Add `develop` branch for feature development workflow
