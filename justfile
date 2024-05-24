default:
	cargo build
	cross build --target aarch64-unknown-linux-gnu

curl:
	curl http://127.0.0.1:3100/common-name-to-scientific-name.json
	curl http://127.0.0.1:3100/by-day-and-common-name.json
	curl http://127.0.0.1:3100/by-common-name.json
	curl http://127.0.0.1:3100/American%20Crow/files.json
