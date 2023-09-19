#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::{error::Error, ffi::CString, io, io::prelude::*, str};
use core::ffi::CStr;
use nix::{
    fcntl::{open, OFlag},
    libc::{STDIN_FILENO, STDOUT_FILENO},
    sys::{stat::Mode, wait::waitpid},
    unistd::{close, dup2, execvp, fork, pipe, ForkResult},
};

#[derive(Debug, PartialEq, Eq)]
pub struct Command {
    pub command: Vec<CString>,
    pub stdin: Option<CString>,
    pub stdout: Option<CString>,
}

impl Command {
    fn run(&self) {
        let mut args = Vec::<CString>::new();
            for arg in &self.command {
                args.push(arg.clone());
            }
            if args.len() != 0 {
                self.setup_stdin();
                self.setup_stdout();
        
                match execvp(&args[0], args.as_slice()) {
                    Ok(_) => println!("ok finished"),
                    Err(err) => println!("failed finished: {err}"),
                };
            }
    }

    fn setup_stdout(&self) {
        if self.stdout.is_some() {
            let path = self.stdout.as_ref().unwrap();
            let fd = open(path.as_c_str(), 
                OFlag::O_WRONLY | OFlag::O_CREAT,
                Mode::S_IRWXU).unwrap();
    
            dup2(fd, STDOUT_FILENO).unwrap();
    
            close(fd).unwrap();
        }
    }

    fn setup_stdin(&self) {
        if self.stdin.is_some() {
            let path = self.stdin.as_ref().unwrap();
            let fd = open(path.as_c_str(), 
                OFlag::O_WRONLY | OFlag::O_CREAT,
                Mode::S_IRWXU).unwrap();
    
            dup2(fd, STDIN_FILENO).unwrap();
    
            close(fd).unwrap();
        }
    }
}

fn starts(word: &str, sym: char) -> bool {
    word.len() > 1 && word.chars().next().unwrap() == sym
}

pub fn parse(line: &str) -> Vec<Command> {
    let mut result = Vec::<Command>::new();
    let mut command = Command{
        command: Vec::<CString>::new(),
        stdin: None,
        stdout: None,
    };

    let mut next_stdin = false;
    let mut next_stdout = false;
    
    for word in line.trim().split_whitespace() {
        let mut new_word = word;
        let starts_stdin = starts(word, '<');
        let starts_stdout = starts(word, '>');

        if word == "<" || starts_stdin {
            
            next_stdin = true;
            if starts_stdin {
                new_word = &word[1..];
            } else {
                continue;
            }
        } else if word == ">" || starts_stdout {
            next_stdout = true;
            if word.len() > 1 && starts_stdout {
                new_word = &word[1..];
            } else {
                continue;
            }
        } else if word == "|" {
            result.push(command);

            command = Command{
                command: Vec::<CString>::new(),
                stdin: None,
                stdout: None,
            };
            continue;     
        }

        if next_stdin {
            // println!("new stdin: {new_word}");
            command.stdin = Some(CString::new(new_word).unwrap());
            next_stdin = false;
        } else if next_stdout {
            // println!("new stdout: {new_word}");
            command.stdout = Some(CString::new(new_word).unwrap());
            next_stdout = false;
        } else {
            command.command.push(CString::new(new_word).unwrap());
        }
    }

    result.push(command);

    result
}

fn fork_and_run(commands: &[Command], mut pos: usize) {
    let command = &commands[pos];

    pos += 1;
    if pos == commands.len() {
        command.run();
    } else {
        let (inpipe, outpipe) = pipe().unwrap();
        
        let result_fork_other = unsafe {fork()};
        match result_fork_other {
            Ok(ForkResult::Parent {child}) => {
                let child_other = child;

                let result_fork_current = unsafe {fork()};
                match result_fork_current {
                    Ok(ForkResult::Parent {child}) => {
                        close(inpipe).unwrap();
                        close(outpipe).unwrap();

                        waitpid(child, None).unwrap();
                        waitpid(child_other, None).unwrap();
                    },
                    Ok(ForkResult::Child) => {
                        close(inpipe).unwrap();
                        match dup2(outpipe, STDOUT_FILENO) {
                            Ok(_) => println!("ok dup2"),
                            Err(err) => println!("failed dup2: {err}"),
                        }
                        close(outpipe).unwrap();
        
                        command.run();
                    },
                    Err(_) => println!("fork failed"),
                }
            },
            Ok(ForkResult::Child) => {
                close(outpipe).unwrap();
                dup2(inpipe, STDIN_FILENO).unwrap();
                close(inpipe).unwrap();

                fork_and_run(commands, pos);
            },
            Err(_) => println!("fork failed"),
        }
    }
}

pub fn main() -> Result<(), Box<dyn Error>> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        let commands = parse(&line);

        match unsafe {fork()} {
            Ok(ForkResult::Parent {child}) => {
                waitpid(child, None).unwrap();
            },
            Ok(ForkResult::Child) => {
                
                fork_and_run(&commands[0..], 0);
            },
            Err(_) => println!("fork failed"),
        }
    }

    println!("finished");
    Ok(())
}