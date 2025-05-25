docker-build:
	docker build . -t maelstrom-container

docker-run:
	cargo build --manifest-path=tea/Cargo.toml && \
		docker run --name maelstrom --rm -v ./tea/target/:/builds/ -v ./debug-logs:/store/ maelstrom-container \
		test -w g-counter --bin /builds/debug/gcounter --node-count 3 --rate 100 --time-limit 20 --nemesis partition

docker-stop: 
	docker stop maelstrom

docker-run-detached:
	docker run --name maelstrom --rm -d -v ./tea/target/:/builds/ maelstrom-container
