.PHONY: dev test test-web test-server

dev:
	@trap 'kill 0' INT TERM EXIT; \
	cargo run --manifest-path server/Cargo.toml & \
	npm run dev:web & \
	wait

test:
	npm test

test-web:
	npm run test:web

test-server:
	npm run test:server
