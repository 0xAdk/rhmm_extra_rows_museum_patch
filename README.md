# Looped museum patch

A patch for rhmm that loops the museum scene vertically.

This was made as an exercise in generating ips patches for Luma3DS's game patching feature, and after for setting up code to inject compiled rust code.

## requirements
patching code.bin:
- [armips](https://github.com/Kingcom/armips)
- [flips](https://github.com/Alcaro/Flips)
- rhmm's code.bin and exheader.bin in `input`

building injection.bin:
- the [arm gnu toolchain](https://developer.arm.com/Tools%20and%20Software/GNU%20Toolchain)
- [rust](https://www.rust-lang.org/tools/install)
    - `nightly` + with the `rust-src` component installed

## building and installing
After all the requirements are fulfilled run
```sh
export INJECTION_PROFILE=release
make
```

This will create:
- output/code.ips
- output/exheader.ips
- output/injection.bin

to then install, copy these files into `/luma/titles/000400000018A400` on your
3DS

---
or alternatively you can use the `ftp_send_files.sh` helper script.
```sh
export FTP_HOST=192.168.0.51 INJECTION_PROFILE=release
make all send
```

### ftp_send_files.sh


it requires:
- the `ftp` command is available in path (on my system this is provided by [inetutils](https://archlinux.org/packages/core/x86_64/inetutils/))
- you have an FTP server running on the 3DS (This can be achieve through either ftpd or 3DShell)
- you set the environment variable `FTP_HOST` to your 3DS' ip address before running the script.
   for example:
   ```sh
   export FTP_HOST=192.168.0.51
   ./ftp_send_files.sh # or `make send`
   ```
