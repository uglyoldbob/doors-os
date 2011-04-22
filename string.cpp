//string.cpp
//this handles all string operations for the kernel
#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

EXTERNC int strlen(char *string)
{
	int counter;
	for (counter = 0; string[counter] != '\0'; counter++) {};
	return counter;
}

EXTERNC int strlenw(unsigned short *string)
{	//works on two-byte characters
	int counter;
	for (counter = 0; string[counter] != 0xFFFF; counter++) {};
	return counter;
}

EXTERNC char *strcpy(char *destination, const char *source )
{
	int counter = 0;
	do
	{
		destination[counter] = source[counter];
		counter++;
	} while (source[counter - 1] != 0);
	return destination;
}

EXTERNC short *strcpyw(short *destination, const short *source)
{	//works on two-byte characters
	int counter = 0;
	do
	{
		destination[counter] = source[counter];
		counter++;
	} while (source[counter - 1] != 0);
	return destination;
}

EXTERNC unsigned int stringCompare(const char *a, const char *b)
{	//TODO: lookup the specifications for strcmp and implement it
	for(unsigned int bob = 0; ( (a[bob] != '\0') || (b[bob] != '\0') );bob++)
	{
		if (a[bob] != b[bob])
			return -1;
	}
	return 0;
}

EXTERNC unsigned short *precatenatew(unsigned short *original, unsigned short *insert)
{
	unsigned short *ret_val;
	unsigned int a, b;
	ret_val = new unsigned short[strlenw(insert) + strlenw(original) + 1];
		//declare enough size to place both string and a terminator into it
	for (a = 0; insert[a] != 0xFFFF; a++)
	{
		ret_val[a] = insert[a];
	}
	for (b = 0; original[b] != 0xFFFF; b++, a++)
	{
		ret_val[a] = original[a];
	}
	ret_val[a] = 0xFFFF;
	return ret_val;	
}
