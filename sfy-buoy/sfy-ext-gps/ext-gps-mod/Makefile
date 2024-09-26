.DEFAULT_GOAL := bin

dep-arduion-cli:
	curl -fsSL https://raw.githubusercontent.com/arduino/arduino-cli/master/install.sh | BINDIR=/usr/local/bin sh

dependencies:
	arduino-cli config init
	arduino-cli config add board_manager.additional_urls https://raw.githubusercontent.com/sparkfun/Arduino_Apollo3/main/package_sparkfun_apollo3_index.json
	arduino-cli core update-index
	arduino-cli core install "Sparkfun:apollo3@1.2.1"
	arduino-cli lib update-index
	arduino-cli lib install "SparkFun u-blox GNSS v3"

build-docker:
	docker build -t sfy-rtk .

run-docker:
	docker run -it --rm --name sr -v $(shell pwd)/:/ext-gps-mod sfy-rtk

bin:
	arduino-cli compile -e --output-dir build --build-cache-path /tmp/build -v -b SparkFun:apollo3:artemis .
