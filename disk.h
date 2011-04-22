void examine_ide();

struct disk_info
{	//unsigned char is used becuase it is short
	unsigned char power_status;	//active, idle, standby, sleep, not used
	unsigned char busy;	//is this drive busy
	
};
