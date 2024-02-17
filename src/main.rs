use futures::Future;
use futures::FutureExt;
use rand::Rng;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::time::Duration;
use std::{thread, time};

const N: u16 = 5;

#[derive(Debug, Clone, PartialEq)]
enum State {
    THINKING = 0, // philosopher is THINKING
    HUNGRY = 1,   // philosopher is trying to get forks
    EATING = 2,   // philosopher is EATING
}

#[derive(Debug, Default)]
struct DiningPhilosphersTable {
    num_seats: u16,
    seats: Vec<State>,
    critical_region_mtx: Mutex<u16>, // (picking up and putting down the forks)
    output_mtx: Mutex<u16>, // for synchronized cout (printing THINKING/HUNGRY/EATING status)
    both_forks_available: Vec<(Mutex<i16>, Condvar)>, // simulate binary semaphore with mutex and cond var
                                                      //both_forks_available_permit: Vec<Option<SemaphorePermit<'a>>>,
}

struct DiningPhilosphersTableBuilder {}

impl DiningPhilosphersTableBuilder {
    fn new(num_seats: u16) -> DiningPhilosphersTable {
        let mut table = DiningPhilosphersTable {
            num_seats: num_seats,
            seats: vec![State::THINKING; N.into()],
            ..Default::default()
        };
        for _i in 0..N {
            let sem: (Mutex<i16>, Condvar) = (Mutex::<i16>::new(0), Condvar::new());
            table.both_forks_available.push(sem);
        }
        table
    }
}

impl DiningPhilosphersTable {
    fn left(&self, i: u16) -> u16 {
        // number of the left neighbor of philosopher i, for whom both forks are available
        let ret = (i + N - 1) % N; // N is added for the case when  i - 1 is negative
        ret
    }

    fn right(&self, i: u16) -> u16 {
        // number of the right neighbour of the philosopher i, for whom both forks are available
        let ret = (i + 1) % N;
        ret
    }

    fn my_rand(&self, min: u16, max: u16) -> u16 {
        let mut rng = rand::thread_rng();
        rng.gen_range(min..max - 1)
    }

    fn test(&mut self, i: u16) {
        // if philosopher i is hungry and both neighbours are not eating then eat
        // i: philosopher number, from 0 to N-
        if self.seats[i as usize] == State::HUNGRY
            && self.seats[self.left(i) as usize] != State::EATING
            && self.seats[self.right(i) as usize] != State::EATING
        {
            self.seats[i as usize] = State::EATING;

            let (mutex, condvar) = &self.both_forks_available[i as usize];
            let mut started = mutex.lock().unwrap();
            *started = *started + 1;

            condvar.notify_all();
        }
    }

    fn think(&self, i: u16) {
        let duration = self.my_rand(400, 800);
        {
            let mut _output_mtx = self.output_mtx.lock().unwrap();
            // critical section for uninterrupted print
            println!("{:?} is thinking {:?} ms", i, duration);
        }
        thread::sleep(time::Duration::from_millis(duration.into()));
    }

    fn take_forks(&mut self, i: u16) {
        let _critical_region_mtx = Box::new(self.critical_region_mtx.lock().unwrap()).as_mut(); // enter critical region
        self.seats[i as usize] = State::HUNGRY; // record fact that philosopher i is State::HUNGRY
        {
            let mut _output_mtx = self.output_mtx.lock().unwrap(); // critical section for uninterrupted print
            println!("\t\tphilosopher {:?} is State::HUNGRY", i);
        }
        self.test(i); // try to acquire (a permit for) 2 forks

        // exit critical region

        let (lock, cvar) = &self.both_forks_available[i as usize];
        let mut started = lock.lock().unwrap();
        while *started <= 0 {
            started = cvar.wait(started).unwrap();
        }
        *started = *started - 1;
    }

    fn eat(&self, i: u16) {
        let duration = self.my_rand(400, 800);
        {
            let mut _output_mtx = self.output_mtx.lock().unwrap(); // critical section for uninterrupted print
            println!(
                "\t\t\t\tphilosopher {:?} is eating duration {:?} ms",
                i, duration
            );
        }
        thread::sleep(time::Duration::from_millis(duration.into()));
    }

    fn put_forks(&mut self, i: u16) {
        let _critical_region_mtx = Box::new(self.critical_region_mtx.lock().unwrap()).as_mut(); // enter critical region
        self.seats[i as usize] = State::THINKING; // philosopher has finished State::EATING
        let left = self.left(i);
        let right = self.right(i);
        self.test(left); // see if left neighbor can now eat
        self.test(right); // see if right neighbor can now eat
                          // exit critical region by exiting the function
    }

    fn philosopher(&mut self, i: u16) {
        self.think(i); // philosopher is State::THINKING
        self.take_forks(i); // acquire two forks or block
        self.eat(i); // yum-yum, spaghetti
        self.put_forks(i); // put both forks back on table and check if neighbours can eat
    }
}

fn launch_threads() {
    let my_table = DiningPhilosphersTableBuilder::new(N);
    let mut threads: Vec<thread::JoinHandle<_>> = Vec::<thread::JoinHandle<_>>::new();
    let table = Arc::new(Mutex::new(my_table));
    for i in 0..N {
        let data = Arc::clone(&table);
        threads.push(thread::spawn(move || loop {
            let mut shared = data.lock().unwrap();

            let _ = shared.philosopher(i);

            let duration = shared.my_rand(200, 1600);
            drop(shared);
            thread::sleep(time::Duration::from_millis(duration.into()));
        }));
    }
}

use futures::executor::block_on;

fn main() {
    launch_threads();

    loop {
        thread::sleep(time::Duration::from_millis(5000));
    }
}
