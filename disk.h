extern unsigned long ReadSectors(unsigned long Address, unsigned long SectorNumber, 
						unsigned long NumSectors, unsigned char DriveNum);
	//0 is the first sector, 0 is A:
extern unsigned long ReadSector(unsigned long Address, unsigned long SectorNumber, 
					   unsigned char DriveNum);
	//0 is the first sector, 0 is A: