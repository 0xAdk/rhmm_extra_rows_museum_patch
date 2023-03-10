A (super) simple patch for rhmm that loops the museum scene vertically.

This was made as an exercise in generating ips patches for Luma3DS's game patching feature.

## Patching code.bin
requires:
1. [armips](https://github.com/Kingcom/armips) installed
2. [flips](https://github.com/Alcaro/Flips) installed
3. An unencrypted code.bin placed in `input` as `code.bin`

then just run `make` and it should output:
1. a patched `code.bin` under `output/code.bin`
2. a `code.ips` file under `output/code.ips`


## ftp_send_ips.sh
An `./ftp_send_ips.sh` script is provided for convenience.

It requires:
- the `ftp` command is available in path.  
   on my system this is provided by [inetutils](https://archlinux.org/packages/core/x86_64/inetutils/)
- your 3ds has an ftp sever running
- you set the environment variable `FTP_HOST` to your 3ds' ip address before running the script.
   For example:
   ```
   export FTP_HOST=192.168.0.51
   ```
- the path `/luma/titles/000400000018A400` already exists on your 3ds for the script to copy the ips file into.
