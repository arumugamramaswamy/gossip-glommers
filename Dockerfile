FROM rust:latest

RUN apt-get update && \
      apt-get -y install graphviz gnuplot wget default-jdk

RUN wget https://github.com/jepsen-io/maelstrom/releases/download/v0.2.3/maelstrom.tar.bz2
RUN tar -xf maelstrom.tar.bz2

ENTRYPOINT ["./maelstrom/maelstrom"]

