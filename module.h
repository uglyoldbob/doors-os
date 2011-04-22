//module.h

//licensing info
//version info
//name, purpose

//MODULE HANDLING
//load module
//unload module
//reference count
	//with a list of modules using this currently
	//modules waiting for this to be unloaded
//state: coming, going, living (as linux kernel module.h calls them)

//MODULE CODE
//initialization (startup)
//destruction (exit)
//various methods offered by the module
//kernel modules used by the module
//exception tables?

struct module
{
	//constructor
	int (*init) (void);
	//destructor
	void (*exit) (void);
};
