default:
	cargo build


curl:
	curl http://127.0.0.1:3000/common-name-to-scientific-name.json
	curl http://127.0.0.1:3000/by-day-and-common-name.json
	curl http://127.0.0.1:3000/by-common-name.json
	curl http://127.0.0.1:3000/American%20Crow/files.json
