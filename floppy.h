#ifndef FLOPPY_H
#define FLOPPY_H
unsigned long SIZE_LONG = sizeof(unsigned long);
unsigned long SIZE_CHAR = sizeof(unsigned char);
extern bool ReadSectors(unsigned long SectorNumber, unsigned long NumSectors, unsigned char DriveNum);
	//0 is the first sector, 0 is A:
extern bool ReadSector(unsigned long SectorNumber, unsigned char DriveNum);
	//0 is the first sector, 0 is A:


#endif
