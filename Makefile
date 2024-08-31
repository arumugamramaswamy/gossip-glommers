docker-build:
	docker build . -t maelstrom-container

docker-run:
	cargo build --manifest-path=tea/Cargo.toml && \
		docker run --name maelstrom --rm -v ./tea/target/:/builds/ -v ./debug-logs:/store/ maelstrom-container \
		test -w unique-ids --bin /builds/debug/generate --time-limit 30 --rate 1000 --node-count 3 --availability total --nemesis partition

docker-stop: 
	docker stop maelstrom

docker-run-detached:
	docker run --name maelstrom --rm -d -v ./tea/target/:/builds/ maelstrom-container
