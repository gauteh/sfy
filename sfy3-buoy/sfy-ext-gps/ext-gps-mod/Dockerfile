FROM ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y curl git build-essential ffmpeg arduino-mk && apt-get clean

RUN curl -fsSL https://raw.githubusercontent.com/arduino/arduino-cli/master/install.sh | BINDIR=/usr/local/bin sh

WORKDIR /ext-gps-mod

ADD Makefile .

RUN make dependencies

