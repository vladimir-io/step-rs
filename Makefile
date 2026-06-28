.PHONY: test test-web cli inspect analyze wasm web serve

test:
	cargo test --workspace

test-web: wasm
	cd web && ln -sf ../samples samples 2>/dev/null || true
	cd web && npm install --silent 2>/dev/null || true
	cd web && npx playwright install chromium 2>/dev/null || true
	@pkill -f "python3 -m http.server 8765" 2>/dev/null || true
	cd web && python3 -m http.server 8765 >/dev/null 2>&1 & echo $$! > .test-server.pid
	@sleep 1
	cd web && node e2e.mjs http://localhost:8765
	@if [ -f web/.test-server.pid ]; then kill $$(cat web/.test-server.pid) 2>/dev/null || true; rm -f web/.test-server.pid; fi

cli:
	cargo build -p steprs-cli --release

inspect:
	cargo run -p steprs-cli -- inspect samples/cylinder_block.step

analyze:
	cargo run -p steprs-cli -- analyze samples/cylinder_block.step

wasm:
	cd web && bash build-wasm.sh

web: wasm
	cd web && ln -sf ../samples samples 2>/dev/null || true && python3 -m http.server 8080

serve: web
