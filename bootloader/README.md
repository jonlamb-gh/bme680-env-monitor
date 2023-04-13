# bootloader

First idea was to have a high-functioning bootloader and app in flash.
App would set RAM bits and reset and bootloader would run and do network IO
so you'd reset into bootloader, then upload new fw, then reset

that's doable, but there's enough FLASH to implement a more
new idea
real-world scenario, keep the app fm running until ready to switch/upgrade

flash is split up into 3 sections, bootloader and 2 app fw "slots"
bootloader is dumb, just decides which app fw slot to boot, and maybe some sanity checks?

app fw implements the fw update protocol
knows how to write to the "other" slot for an update, crc/sanity check it
then set a FLASH bit saying swap slots or w/e
then reset
bootloader boots into new slot
maybe new upgraded firmware has some protocol/process to ACK
and confirm that it's good to go
maybe the first part is done in RAM sticky bits and only FLASH persist
once new fw says "yes, I'm good to go"


Assumptions:
* bootloader (and configs??) will fit in sections 0..=3 (64K)
  - need some config to say which app fm slot to boot, maybe last page or ? in sector 3
* application will fit in <= 194K (sectors 4, 5 for slot 0 are smallest)

## Memory Map

NOTE: K = KiBi = 1024 bytes

| Sector | Address     | Size  | Function |
| :---:  | :---:       | :---: | :---:    |
| 0      | 0x0800_0000 | 16K   | bootloader firmware |
| 1      | 0x0800_4000 | 16K   | bootloader firmware |
| 2      | 0x0800_8000 | 16K   | bootloader firmware |
| 3      | 0x0800_C000 | 16K   | bootloader firmware |
| 4      | 0x0801_0000 | 64K   | application firmware slot 0 |
| 5      | 0x0802_0000 | 128K  | application firmware slot 0 |
| 6      | 0x0804_0000 | 128K  | application firmware slot 1 |
| 7      | 0x0806_0000 | 128K  | application firmware slot 1 |
