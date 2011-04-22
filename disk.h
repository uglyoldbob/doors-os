struct sectorReturn
{	//this is used to send and recieve sectors
	unsigned char *data;
	unsigned long size;	//the size of the buffer
};

struct driveData
{	//stores data required to access a drive
	char *name;					//not case sensitive (I think case sensitivity here is a bad idea)
	unsigned char drive_num;	//always present and useful
	
	//function pointer to the function selector
};

struct sectorReturn readSector(unsigned char driveNum, unsigned long sectorNumber);

///////////////////////////////////////////////////////////////////////////////////////
void examine_ide();

struct disk_info
{	//unsigned char is used becuase it is short
	unsigned char power_status;	//active, idle, standby, sleep, not used
	unsigned char busy;	//is this drive busy
	
};

