output/code.ips: output/code.bin
	flips --create --ips input/code.bin output/code.bin output/code.ips

output/code.bin: input/code.bin asm/main.s
	armips asm/main.s
