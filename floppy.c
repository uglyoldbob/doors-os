#include "floppy.h"
#include "memory.h"
#include "dma.h"
extern unsigned int inportb(unsigned int port);		//entrance.asm
extern unsigned int outportb(unsigned int value, unsigned int port);	//entrance.asm
extern int WaitFloppyInt();	//entrance.asm

extern unsigned int timer;	//entrance.asm

unsigned char driveA;
unsigned char driveB;

	//waits for the floppy drive to signal with an interrupt
//actually only the bottom two bytes of port and the bottom byte of the return value is used

//offsets for the various registers
#define STATUS_REG_A            0x0000 /*PS2 SYSTEMS*/
#define STATUS_REG_B            0x0001 /*PS2 SYSTEMS*/
#define DIGITAL_OUTPUT_REG      0x0002
#define TAPE_DRIVE_REGISTER     0x0003
#define MAIN_STATUS_REG         0x0004
#define DATA_RATE_SELECT_REG    0x0004 /*PS2 SYSTEMS*/
#define DATA_REGISTER           0x0005
#define DIGITAL_INPUT_REG       0x0007 /*AT SYSTEMS*/
#define CONFIG_CONTROL_REG      0x0007 /*AT SYSTEMS*/
#define PRIMARY_RESULT_STATUS   0x0000
#define SECONDARY_RESULT_STATUS 0x0000

//command bytes for the floppy disk controller
#define FIX_DRIVE_DATA          0x03
#define CHECK_DRIVE_STATUS      0x04
#define CALIBRATE_DRIVE         0x07
#define CHECK_INTERRUPT_STATUS  0x08
#define FORMAT_TRACK            0x4D
#define READ_SECTOR             0x66
#define READ_DELETE_SECTOR      0xCC
#define READ_SECTOR_ID          0x4A
#define READ_TRACK              0x42
#define SEEK_TRACK              0x0F
#define WRITE_SECTOR            0xC5
#define WRITE_DELETE_SECTOR     0xC9

#define DISK_PARAMETER_ADDRESS 0x000FEFC7 /* location where disk parameters */
                                          /* is stored by bios */

typedef struct
{	//the table as BIOS fills it out at the above address
  unsigned char steprate_headunload;
  unsigned char headload_ndma;
  unsigned char motor_delay_off; /*specified in clock ticks*/
  unsigned char bytes_per_sector;
  unsigned char sectors_per_track;
  unsigned char gap_length;
  unsigned char data_length; /*used only when bytes per sector == 0*/
  unsigned char format_gap_length;
  unsigned char filler;
  unsigned char head_settle_time; /*specified in milliseconds*/
  unsigned char motor_start_time; /*specified in 1/8 seconds*/
}__attribute__ ((packed)) floppy_parameters;

floppy_parameters floppy_disk; 
/*declare variable of floppy_parameters type*/
//will be used for all future floppy disk access
//this structure is loaded when initialize_floppy is called
//it is loaded with information taken directly from what is setup when the computer booted up

void waitRecieveFloppy(unsigned int base)
{
	//while ((inportb(base + CHECK_DRIVE_STATUS) & 0xC0) != 0xC0){};
	unsigned int temp;
	while (1)
	{
		temp = inportb(base + CHECK_DRIVE_STATUS);
		if ((temp & 0xD0) == 0xD0)	//only let it retrieve data while the controller is not busy?
		{
			return;
		}
		else if ((temp & 0xD0) == 0x80)
		{
			display("Controller is waiting for data, this is unexpected, MSR:");
			PrintNumber(temp);
			display("\n");
		}
		Delay(1000);
	}
}


void sendFloppyCommand(unsigned int base, unsigned char command)
{
	unsigned int temp;
	while (1)
	{
		temp = inportb(base + CHECK_DRIVE_STATUS);
		if ((temp & 0xC0) == 0xC0)
		{
			display("Controller wants to give, this is unexpected, MSR:");
			PrintNumber(temp);
			display("\n");
		}
		else if ((temp & 0xC0) == 0x80)
		{
			outportb(command, base + DATA_REGISTER);
			break;
		}
		display("Problem with sendFloppyCommand\n");
		Delay(1000);
	}
}

void check_floppy_status(unsigned int base, unsigned int *st0, unsigned int *cylinder)
{	//performs the FDC instruction and returns all applicable results
	//waitSendFloppy(base);
	sendFloppyCommand(base, CHECK_INTERRUPT_STATUS);
	waitRecieveFloppy(base);
	*st0 = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*cylinder = inportb(base+DATA_REGISTER);
}

void floppy_configure_drive(unsigned int base)
{
  sendFloppyCommand(base,FIX_DRIVE_DATA);/*config/specify command*/
  sendFloppyCommand(base,floppy_disk.steprate_headunload);
  sendFloppyCommand(base,floppy_disk.headload_ndma);	//set bit 0 for nondma transfer, clear it for DMA transfer
	//no results
}

int floppy_calibrate_drive(unsigned int base,char drive)
{	//drive = 0,1,2,3
	unsigned int st0, cylinder, command;
	command = (0x10<<drive);
	command += 0x0C + drive;
	outportb(command, base + DIGITAL_OUTPUT_REG);
	//make sure motor is turned on
	do
	{
		sendFloppyCommand(base,CALIBRATE_DRIVE); /*Calibrate drive*/
  	sendFloppyCommand(base,drive);
		if (WaitFloppyInt() == -1)
			return -1;
  	check_floppy_status(base,&st0,&cylinder); /*check interrupt status and
                                                store results in global variables
                                                st0 and cylinder*/
		if (st0 != 0x20)
		{
			display("Error in calibrate drive, st0:");
			PrintNumber(st0);
			return -1;
		}
	} while (cylinder != 0);
	//repeat until the floppy drive is over cylinder 0
	return 0;
}

int floppy_seek_to_cylinder(unsigned int cylinder, unsigned int head, unsigned int base, unsigned char drive)
{
	unsigned int st0, cylinder_check;
	do
	{
		sendFloppyCommand(base, SEEK_TRACK);
		sendFloppyCommand(base, head<<2 | drive);
		sendFloppyCommand(base, cylinder);
		if (WaitFloppyInt() == -1)
			return -1;	//wait for the impending interrupt or a timeout
		check_floppy_status(base, &st0, &cylinder_check);
	} while (cylinder_check != cylinder);

	return 0;
}

int reset_floppy(unsigned int base, char drive)
{
	unsigned int st0, cylinder;	//this will be used for any necessary storage of FDC states
	//reset the floppy disk to a known state
	outportb(0, base + DIGITAL_OUTPUT_REG);
	Delay(100);
	//	while (inportb(base + CHECK_DRIVE_STATUS) != 0);
	//should delay int enough for the reset to finish course
	outportb(0x0C, base + DIGITAL_OUTPUT_REG);
	if (WaitFloppyInt() == -1)
			return -1;
	check_floppy_status(base, &st0, &cylinder);
	check_floppy_status(base, &st0, &cylinder);
	check_floppy_status(base, &st0, &cylinder);
	check_floppy_status(base, &st0, &cylinder);
	outportb(0, base + CONFIG_CONTROL_REG);
	floppy_configure_drive(base);
	if (floppy_calibrate_drive(base, drive) == -1)
		return -1;
	return 0;
}

void floppy_getResults(unsigned int *st0, unsigned int *st1, unsigned int *st2, unsigned int *cylinder_r, 
					unsigned int *head_r, unsigned int *sector_r, unsigned int *size_r, unsigned int base)
{	//recieves the results from the floppy drive after a sector/track read/write command
	waitRecieveFloppy(base);
	*st0 = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*st1 = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*st2 = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*cylinder_r = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*head_r = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*sector_r = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*size_r = inportb(base+DATA_REGISTER);
}

int floppy_read_track(unsigned int sector_number, unsigned char drive,unsigned int buffer, unsigned int base)
{	//doesn't seem to work properly on bochs
	unsigned int st0, st1, st2, cylinder_r, head_r, sector_r, size_r;//, cylinder_check;
	//these store the results from the read command
	unsigned int length = 1;
	unsigned int counter = 0;
	unsigned int command;
	unsigned int cylinder, head, sector;
	sector = (sector_number % floppy_disk.sectors_per_track) + 1;
	cylinder = (sector_number / floppy_disk.sectors_per_track) / 2;	//there are 2 heads on a floppy drive
	head = (sector_number / floppy_disk.sectors_per_track) % 2;			//2 heads on a floppy drive
	for (counter = 0; counter < floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
	length *= floppy_disk.sectors_per_track;	//bytes per track
		//size of the sector in bytes
		//will need to be recalculated
	//enable the motor first
	command = (0x10<<drive);
	command += 0x0C + drive;
	outportb(command, DIGITAL_OUTPUT_REG);
	//configure the DMA (channel 2)
	startDMA(buffer, length - 1, 2, 0x45);
	//give the seek track command
	floppy_seek_to_cylinder(cylinder, head, base, drive);
	Delay(floppy_disk.head_settle_time);
	//the length of time in milliseconds it takes for the head to settle after moving
	sendFloppyCommand(base, READ_TRACK);
	sendFloppyCommand(base, head<<2|drive);
	sendFloppyCommand(base, cylinder);
	sendFloppyCommand(base, head);
	sendFloppyCommand(base, sector);
	sendFloppyCommand(base, floppy_disk.bytes_per_sector);  /*sector size = 128*2^size*/
	sendFloppyCommand(base, floppy_disk.sectors_per_track); /*last sector*/
	sendFloppyCommand(base, floppy_disk.gap_length);        /*27 default gap3 value*/
	sendFloppyCommand(base, floppy_disk.data_length);       /*default value for data length*/
	if (WaitFloppyInt() == -1)
			return -1;	//wait for the completion of the command or a timeout
	//check_floppy_status(base, &st0, &cylinder);
	floppy_getResults(&st0, &st1, &st2, &cylinder_r, &head_r, &sector_r, &size_r, base);
	return 0;
}


int floppy_read_sector(unsigned int sector_number, unsigned char drive,unsigned int buffer, unsigned int base)
{	//starts at sector 0 for sector_number
	//this should be modified to use the same buffer every time
	//and copy the data to the requested buffer space
	//drive = 00, 01, 02, 03
	unsigned int st0, st1, st2, cylinder_r, head_r, sector_r, size_r;//, cylinder_check;
	//these store the results from the read command
	unsigned int length = 1;
	unsigned int counter = 0;
	unsigned int command;
	unsigned int cylinder, head, sector;
	sector = (sector_number % floppy_disk.sectors_per_track) + 1;
	cylinder = (sector_number / floppy_disk.sectors_per_track) / 2;	//there are 2 heads on a floppy drive
	head = (sector_number / floppy_disk.sectors_per_track) % 2;			//2 heads on a floppy drive
	for (counter = 0; counter < floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
	sector_size = length;
		//size of the sector in bytes
	//set floppy disk status to known state
	if (reset_floppy(base, drive) == -1)
		return -1;
	//enable the motor for the drive (reset_floppy should do this already)
//	command = (0x10<<drive);
//	command += 0x0C + drive;
//	outportb(command, DIGITAL_OUTPUT_REG);
	//configure the DMA (channel 2)
	startDMA(sector_buffer, length - 1, 2, 0x45);	//dma always copies at least 1 byte
	//give the seek track command
	if (floppy_seek_to_cylinder(cylinder, head, base, drive) == -1)
		return -1;
	Delay(floppy_disk.head_settle_time);
	//the length of time in milliseconds it takes for the head to settle after moving
	sendFloppyCommand(base, READ_SECTOR);
	sendFloppyCommand(base, head<<2|drive);
	sendFloppyCommand(base, cylinder);
	sendFloppyCommand(base, head);
	sendFloppyCommand(base, sector);
	sendFloppyCommand(base, floppy_disk.bytes_per_sector);  /*sector size = 128*2^size*/
	sendFloppyCommand(base, floppy_disk.sectors_per_track); /*last sector*/
	sendFloppyCommand(base, floppy_disk.gap_length);        /*27 default gap3 value*/
	sendFloppyCommand(base, floppy_disk.data_length);       /*default value for data length*/
	if (WaitFloppyInt() == -1)
			return -1;	//wait for the completion of the command
	//check_floppy_status(base, &st0, &cylinder);
	floppy_getResults(&st0, &st1, &st2, &cylinder_r, &head_r, &sector_r, &size_r, base);
	memcopy(buffer, sector_buffer, length);
	//turn off floppy disk motor
	outportb(0, base + DIGITAL_OUTPUT_REG);
	return 0;
}

void initialize_floppy()
{	//checks for the existence of floppy drives first
	unsigned int check = 0;
	unsigned int length = 0;
	unsigned int counter;
	//new detection routine
	for (counter = 0; counter< 4; counter++)
	{
		if (reset_floppy(FLOPPY_PRIMARY_BASE, counter) == 0)
		{
			display("Floppy drive ");
			PrintNumber(counter);
			display(" is present on primary controller\n");
			outportb(0, FLOPPY_PRIMARY_BASE + DIGITAL_OUTPUT_REG);
		}
		if (reset_floppy(FLOPPY_SECONDARY_BASE, counter) == 0)
		{
			display("Floppy drive ");
			PrintNumber(counter);
			display(" is present on secondary controller\n");
			outportb(0, FLOPPY_SECONDARY_BASE + DIGITAL_OUTPUT_REG);
		}
	}
	display("\n");
	//old detection routine, doesn't work on all computers properly
//	outportb(0x10, 0x70);	//send command to port 0x70
//	check = inportb(0x71);
//	driveA = (check & 0xF0)>>4;
//	driveB = (check & 0xF);
//	display("\tFirst drive code: ");
//	PrintNumber(driveA);
//	display("\n\tSecond drive code: ");
//	PrintNumber(driveB);
//	display("\n");
	memcopy(&floppy_disk, (unsigned char *)DISK_PARAMETER_ADDRESS, sizeof(floppy_parameters));
	//initialize the floppy disk information structure
//	reset_floppy(base, 0);
//	outportb(0, base + DIGITAL_OUTPUT_REG);
	//determine how large a sector is and allocate space for one sector
	for (counter = 0; counter < floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
	if (length == 0)
		length = 0x200;
	sector_buffer = (unsigned long)malloc(0x1000);	//0x1000 is the longest sector possible
	sector_size = length;
}

char * assign_drive(unsigned char drive, unsigned char *prefix)
{	//assigns a floppy drive to be accessed by means of "prefix"/...
	//returns the value assigned
	//ex: "/floppya/"
	
}

//add functions to read files from the root directory of the disk
//then multitasking
//virtual memory
//complete keyboard driver
//...
//write functions to write to disk (carefully)
