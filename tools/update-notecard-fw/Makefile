VERSION=3.4.1.15117
FW=fw/notecard-$(VERSION).bin
stc=/home/gauteh/.bin/STM32_Programmer_CLI/

download-stcube:
	mkdir -p st
	curl https://www.st.com/content/ccc/resource/technical/software/utility/group0/10/04/c7/eb/76/2e/40/9c/stm32cubeprg-lin_v2-11-0/files/stm32cubeprg-lin_v2-11-0.zip/jcr:content/translations/en.stm32cubeprg-lin_v2-11-0.zip --output-dir st -O
	echo "Install from st/ and add STM32_Programmer_CLI to PATH."

download:
	mkdir -p fw
	[ -f $(FW) ] || curl https://dev.blues.io/firmware/notecard-$(VERSION).bin --output $(FW)

enter-bootload: download
	echo bootloader > /dev/ttyACM0

boot: enter-bootload
	STM32_Programmer_CLI --connect port=ACM0 --write $(FW) 0x08000000 --verify --go 0x08000000
