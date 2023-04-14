MEMORY
{
    /* NOTE K = KiBi = 1024 bytes */
    /* STM32F411CEUx */
    /* FLASH : ORIGIN = 0x08000000, LENGTH = 512K */
    /* RAM : ORIGIN = 0x20000000, LENGTH = 128K */

    /* Bootloader is given the first 4 sectors (16K * 4 = 64K) */
    /* Firmware starts at 0x08010000 (offset 0x10000, sector 4, 512K - 64K = 448K) */
    FLASH : ORIGIN = 0x08010000, LENGTH = 194K

    /* First 8 bytes are reserved for the bootloader sticky flag data */
    /* LENGTH = (128K - 8) = 131064 */
    RAM : ORIGIN = 0x20000008, LENGTH = 131064
}
