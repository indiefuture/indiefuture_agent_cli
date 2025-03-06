
use log::info;

#[derive(Default)]
pub struct ContextMemory {} 



impl ContextMemory {

	pub fn add_frag( &mut self,  frag: MemoryFragment ) {

			info!("add memory fragment to context... ");
		}

}



#[derive(Debug)]
#[derive(Clone)]
pub struct MemoryFragment {}

