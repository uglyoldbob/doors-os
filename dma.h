#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

EXTERNC void startDMA(unsigned int address, unsigned int length, unsigned char channel,
		unsigned char mode);
