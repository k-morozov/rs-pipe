#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::error::Error;

use rs_pipe;

fn main() -> Result<(), Box<dyn Error>> {
    rs_pipe::main()
}
