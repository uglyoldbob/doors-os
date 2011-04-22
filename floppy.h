//the base IO address for floppy drive communication
#define FLOPPY_PRIMARY_BASE     0x03F0
#define FLOPPY_SECONDARY_BASE   0x0370

unsigned int sector_size;	//the size of the sector in bytes

unsigned long sector_buffer;
	//this is used so that a specialized memory allocater will not be required
	//but it is required that this buffer be allocated early so that it can claim a spot in lower memory
