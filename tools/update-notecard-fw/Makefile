VERSION=3.5.1.15259
FW=fw/notecard-$(VERSION).bin
FLASH=/home/gauteh/dev/embedded/STMicroelectronics/STM32Cube/STM32CubeProgrammer/bin/STM32_Programmer_CLI
USB ?= /dev/ttyACM0
STUSB ?= USB1

download-stcube:
	mkdir -p st
	curl https://www.st.com/content/ccc/resource/technical/software/utility/group0/10/04/c7/eb/76/2e/40/9c/stm32cubeprg-lin_v2-11-0/files/stm32cubeprg-lin_v2-11-0.zip/jcr:content/translations/en.stm32cubeprg-lin_v2-11-0.zip --output-dir st -O
	echo "Install from st/ and add STM32_Programmer_CLI to PATH."

download:
	mkdir -p fw
	[ -f $(FW) ] || curl https://dev.blues.io/firmware/notecard-$(VERSION).bin --output $(FW)

enter-bootload: download
	echo bootloader > $(USB) || true
	sleep 3

flash: enter-bootload
	$(FLASH) --connect port=$(STUSB) --write $(FW) 0x08000000 --verify --go 0x08000000
