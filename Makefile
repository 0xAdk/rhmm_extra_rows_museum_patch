# vim: foldmethod=marker foldmarker={{{,}}}

.PHONY: default
default: all

INPUT_DIR ?= input
OUT_DIR   ?= output

# one of (debug, release, release-with-debug-info)
INJECTION_PROFILE ?= debug

# ips targets {{{
IPS_OUTPUT_FILES = $(OUT_DIR)/code.ips $(OUT_DIR)/exheader.ips

INPUT_FILES = $(INPUT_DIR)/code.bin $(INPUT_DIR)/exheader.bin
$(INPUT_FILES):
	$(error [ERROR] missing one or more required files: $(INPUT_FILES))

$(IPS_OUTPUT_FILES:.ips=.bin): loader/main.asm
	@mkdir -p $(OUT_DIR)/temp_files
	armips -temp $(OUT_DIR)/temp_files/main.asm loader/main.asm

$(IPS_OUTPUT_FILES): $(OUT_DIR)/%.ips: $(INPUT_DIR)/%.bin $(OUT_DIR)/%.bin
	flips --create --ips $^ $@
# }}}


# injection target {{{
.PHONY: force_target

# if the profile used to compile the rust binary changes, force the objcopy
SAVED_INJECTION_PROFILE = $(shell cat $(OUT_DIR)/injection_profile 2> /dev/null)
$(OUT_DIR)/injection.bin: $(shell \
	if [ "$(SAVED_INJECTION_PROFILE)" != "$(INJECTION_PROFILE)" ]; then \
		echo "force_target"; \
	fi \
) \


INJECTION_OUTPUT_DIR = injection/target/arm-none-eabihf

$(OUT_DIR)/injection.bin: $(INJECTION_OUTPUT_DIR)/$(INJECTION_PROFILE)/injection
	@echo $(INJECTION_PROFILE) > $(OUT_DIR)/injection_profile
	arm-none-eabi-objcopy -O binary $< $(OUT_DIR)/injection.bin

INJECTION_SRC = $(shell find injection/src -type f)

$(INJECTION_OUTPUT_DIR)/%/injection:
	$(error [ERROR] invalid profile for injection "$(INJECTION_PROFILE)")

$(INJECTION_OUTPUT_DIR)/debug/injection: $(INJECTION_SRC)
	cargo -C injection build --profile dev

$(INJECTION_OUTPUT_DIR)/release/injection: $(INJECTION_SRC)
	cargo -C injection build --profile release

$(INJECTION_OUTPUT_DIR)/release-with-debug-info/injection: $(INJECTION_SRC)
	cargo -C injection build --profile release-with-debug-info

# }}}


# all and clean {{{
.PHONY: all send
all: $(OUT_DIR) $(IPS_OUTPUT_FILES) $(OUT_DIR)/injection.bin

$(OUT_DIR): ; @mkdir -p $@


.PHONY: clean clean_output clean_targets
clean: clean_output clean_targets

clean_output:
	-rm $(IPS_OUTPUT_FILES) $(IPS_OUTPUT_FILES:.ips=.bin)
	-rm -r $(OUT_DIR)/temp_files

	-rm $(OUT_DIR)/injection.bin $(OUT_DIR)/injection_profile

	-rmdir $(OUT_DIR)

clean_targets:
	cargo -C injection clean
# }}}

.PHONY: send
send: ; ./ftp_send_files.sh
