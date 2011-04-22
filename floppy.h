#ifndef _FLOPPY_H_
#define _FLOPPY_H_
#include "disk.h"

int floppy_read_sector(unsigned int sector_number, unsigned char drive,unsigned int buffer, unsigned int base);

struct floppy_parameters
{	//the table as BIOS fills it out at the above address
	unsigned char steprate_headunload;	//these two fields are used for the specify command
	unsigned char headload_ndma;				//these two fields are used for the specify command
	unsigned char motor_delay_off; /*specified in clock ticks*/
	unsigned char bytes_per_sector;
	unsigned char sectors_per_track;
	unsigned char gap_length;
	unsigned char data_length; /*used only when bytes per sector == 0*/
	unsigned char format_gap_length;
	unsigned char filler;
	unsigned char head_settle_time; /*specified in milliseconds*/
	unsigned char motor_start_time; /*specified in 1/8 seconds*/
}__attribute__ ((packed));

struct floppy_information
{
	floppy_parameters floppy_disk; 
		/*declare variable of floppy_parameters type*/
		//will be used for all future floppy disk access
		//this structure is loaded when initialize_floppy is called
		//it is loaded with information taken directly from what is setup when the computer booted up
	unsigned int base;	//this defines the base port for the floppy disk controller
		//as of right now, it is assumed registers for all floppy disk controllers use the same offset from the base
	unsigned int 	drive_number;	//each drive has a number identyifing it, this is an array of those numbers
		//INVALID_DRIVE is defined in disk.h
	unsigned int floppy_number;	//this is used to specify which floppy disk number the drive refers to (0-3)
	unsigned int drive_identifier;	//this is supposed to be used so a disk change can be detected
	//drive settings for initialization
	unsigned char speed;
	unsigned char number_sides;
	unsigned char mfm_selector;
	
};


struct driveData * initialize();

class floppy : public disk
{	//one instance of this class will handle access to all floppy drives
	//for right now I will assume that only one floppy drive can be accessed at a time
		//my assumption may prove to be incorrect later, but it is late and I am tired right now
	public:
		floppy();
		~floppy();
		int read_sector(unsigned int drive_num, unsigned long sector_number, unsigned int *buffer);
		int number_drives();
		int get_drive_number(int drive_order);
		int bytes_per_sector(unsigned int drive_num);
		char *identify_driver();	//returns a string identifying the driver
	private:
		//data
		floppy_information *floppies;
		unsigned long *sector_buffer;	//used when reading sectors from disk as a temporary holding area
																	//this will become an array of buffers if I should decide that
																	//more than floppy drive can be accessed at a time
		unsigned int num_drives;	//the total number of floppy drives accessible by this driver
		unsigned int drive_order;	//this will be used to remember which floppy drive is being accessed (base[drive_order])
		//functions
		int set_drive(unsigned int drive);
		int wait_to_recieve(unsigned int drive_order);
		void send_byte(unsigned int drive_order, unsigned char command);
		void check_floppy_status(unsigned int drive_order, unsigned int *st0, unsigned int *cylinder);
		void specify_drive(unsigned int drive_order);
		int recalibrate_drive(unsigned int drive_order);
		int seek_to_cylinder(unsigned int cylinder, unsigned int head, unsigned int drive_order);
		int reset_floppy(unsigned int drive_number);
		void getResults(unsigned int *st0, unsigned int *st1, unsigned int *st2, unsigned int *cylinder_r, 
					unsigned int *head_r, unsigned int *sector_r, unsigned int *size_r, unsigned int drive_order);
		int read_id(unsigned int drive_order);
		int ccr(unsigned int drive_order);
		int select_drive(unsigned int drive_order);
};

#endif
