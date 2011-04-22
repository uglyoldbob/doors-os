struct sectorReturn
{	//this is used to send and recieve sectors
	unsigned char *data;
	unsigned long size;	//the size of the buffer
};

struct sectorReturn readSector(unsigned char driveNum, unsigned long sectorNumber);

void examine_ide();

struct disk_info
{	//unsigned char is used becuase it is short
	unsigned char power_status;	//active, idle, standby, sleep, not used
	unsigned char busy;	//is this drive busy
	
};
