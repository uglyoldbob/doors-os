#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

//might be architecture specific, not sure though

EXTERNC void startDMA(unsigned int address, unsigned int length, unsigned char channel,
		unsigned char mode);
