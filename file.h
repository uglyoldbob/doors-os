#ifndef _FILE_H_
#define _FILE_H_

#include "disk.h"

class filesystem;

//current position
//buffer location and size
//read buffer, write buffer
//flags
//file name
//file size

//this structure holds vital information
	//it is required to do any work with files
struct krnl_FILE
{
	char *filename;	
	unsigned char permissions;
	unsigned long owner;	//owner information
	unsigned long offset;	//offset into the file
	unsigned long length;	//total length of the file
	unsigned char *buffer;	//a buffer to hold a segment of the file
	unsigned int buffer_length;	//length of the buffer
	unsigned int buffer_offset;	//offset of the beginning of the buffer into the file
	void *fs_spec;	//a filesystem specific pointer regarding the file data
};

//from the linux kernel "fs.h"
struct file_operations 
{
//	struct module *owner;
//	loff_t (*llseek) (struct file *, loff_t, int);
//	ssize_t (*read) (struct file *, char __user *, size_t, loff_t *);
//	ssize_t (*write) (struct file *, const char __user *, size_t, loff_t *);
//	ssize_t (*aio_read) (struct kiocb *, const struct iovec *, unsigned long, loff_t);
//	ssize_t (*aio_write) (struct kiocb *, const struct iovec *, unsigned long, loff_t);
//	int (*readdir) (struct file *, void *, filldir_t);
//	unsigned int (*poll) (struct file *, struct poll_table_struct *);
//	int (*ioctl) (struct inode *, struct file *, unsigned int, unsigned long);
//	long (*unlocked_ioctl) (struct file *, unsigned int, unsigned long);
//	long (*compat_ioctl) (struct file *, unsigned int, unsigned long);
//	int (*mmap) (struct file *, struct vm_area_struct *);
//	int (*open) (struct inode *, struct file *);
//	int (*flush) (struct file *, fl_owner_t id);
//	int (*release) (struct inode *, struct file *);
//	int (*fsync) (struct file *, struct dentry *, int datasync);
//	int (*aio_fsync) (struct kiocb *, int datasync);
//	int (*fasync) (int, struct file *, int);
//	int (*lock) (struct file *, int, struct file_lock *);
//	ssize_t (*sendpage) (struct file *, struct page *, int, size_t, loff_t *, int);
//	unsigned long (*get_unmapped_area)(struct file *, unsigned long, unsigned long, unsigned long, unsigned long);
//	int (*check_flags)(int);
//	int (*dir_notify)(struct file *filp, unsigned long arg);
//	int (*flock) (struct file *, int, struct file_lock *);
//	ssize_t (*splice_write)(struct pipe_inode_info *, struct file *, loff_t *, size_t, unsigned int);
//	ssize_t (*splice_read)(struct file *, loff_t *, struct pipe_inode_info *, size_t, unsigned int);
//	int (*setlease)(struct file *, long, struct file_lock **);
};

#define NUMBER_DEVICE_CLASSES_MAXIMUM 256

//file_operations device_list[NUMBER_DEVICE_CLASSES_MAXIMUM];

//class file
//{
//	public:
//		file();
		krnl_FILE *open(char *filename, unsigned int flags, filesystem *owner);
			//returns success and fills out the file descriptor	
		unsigned int eof(krnl_FILE *descriptor, filesystem *owner);
		unsigned long seek(krnl_FILE *descriptor, unsigned long position, filesystem *owner);
		unsigned char get_b(krnl_FILE *descriptor, filesystem *owner);
		unsigned short get_w(krnl_FILE *descriptor, filesystem *owner);
		unsigned long get_dw(krnl_FILE *descriptor, filesystem *owner);
		int close(unsigned char *filename, unsigned int flags, krnl_FILE &descriptor, filesystem *owner);
			//returns success or failure and nulls out the file descriptor
		unsigned long file_size(krnl_FILE &descriptor);
			//returns the size of the file in bytes
		unsigned long file_size_disk(krnl_FILE &descriptor);
			//returns how much space on the disk the file is taking up
		unsigned long file_permissions(krnl_FILE &descriptor);
			//returns the permissions flag for that file

		//int register_device(char *device_name, unsigned long major, unsigned long minor);
			//(/dev/floppya, 5, 1)
			//need a table so that 5,1 will go to the proper place
//	private:
//		filesystem **filesystems;	//this is an array of filesystem pointers
//		disk **disks;			//array of disk objects
//};
#endif

//each disk subclass (ie floppy, ide hd, ide cd) will handle all devices of that type
	//1 instance of the disk subclass will treat all floppy drives
//each filesystem subclass handles one filesystem only
	//1 instance of a fat filesystem subclass will handle one partition
	//there should be a "dummy" instance of every filesystem supported so that when a filesystem is mounted
		//it should be easy to identify what filesystem it is
		//but it might be easier to do it a different way
