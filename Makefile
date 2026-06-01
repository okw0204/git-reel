.PHONY: dev build test test-web test-server test-e2e

dev:
	@trap 'kill 0' INT TERM EXIT; \
	cargo run --manifest-path server/Cargo.toml & \
	npm run dev:web & \
	wait

build:
	npm --workspace web run build

test:
	npm test

test-web:
	npm run test:web

test-server:
	npm run test:server

test-e2e:
	npm run test:e2e
