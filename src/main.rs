use std::sync::Mutex;
use std::sync::Arc;
use std::sync::Condvar;
//use std::sync::Semaphore;
//use tokio::sync::Semaphore;
use std::{thread, time};
use futures::Future;
use futures::FutureExt;
use rand::Rng;

const N: u16 = 5;

#[derive(Debug, Clone, PartialEq)]
enum State 
{
    THINKING = 0,  // philosopher is THINKING
    HUNGRY = 1,    // philosopher is trying to get forks
    EATING = 2,    // philosopher is EATING
}

#[derive(Debug, Default)]
struct DiningPhilosphersTable {
    num_seats: u16,
    seats: Vec<State>,
    critical_region_mtx: Mutex<u16>,
    //critical_region_mtx: Mutex<>,  // mutual exclusion for critical regions for 
    // (picking up and putting down the forks)
    output_mtx: Mutex<u16>,  // for synchronized cout (printing THINKING/HUNGRY/EATING status)
    both_forks_available: Vec<(Mutex<bool>, Condvar)>,   // simulate binary semaphore with mutex and cond var
    //both_forks_available_permit: Vec<Option<SemaphorePermit<'a>>>,
}

struct DiningPhilosphersTableBuilder {
}

impl DiningPhilosphersTableBuilder {
    fn new(num_seats: u16) -> DiningPhilosphersTable  {
        let mut table = DiningPhilosphersTable{
            num_seats: num_seats,
            //seats: Vec::<State>::with_capacity(N.into()),
            seats: vec![State::THINKING; N.into()],
            //both_forks_available: Vec::<Semaphore>::with_capacity(N.into()),
            //both_forks_available: vec![Semaphore::const_new(0); N.into()],
            ..Default::default()
        };
        for _i in 0..N {
            //let sem = Semaphore::const_new(0);
            let sem: (Mutex<bool>, Condvar) = (Mutex::<bool>::new(false), Condvar::new());
            table.both_forks_available.push(sem);
            //table.both_forks_available_permit.push(None);
        }
        table
    }
}

impl DiningPhilosphersTable  {

    fn left(&self, i: u16) -> u16 {
        println!("left for {:?}", i);  
        // number of the left neighbor of philosopher i, for whom both forks are available
        let ret = (i + N - 1) % N; // N is added for the case when  i - 1 is negative
        println!("left for {:?} return {:?}", i, ret); 
        ret
    }

    fn right(&self, i: u16) -> u16 {
        println!("right for {:?}", i); 
        // number of the right neighbour of the philosopher i, for whom both forks are available
        let ret = (i + 1) % N;
        println!("right for {:?} return {:?}", i, ret); 
        ret
    }

    fn my_rand(&self, min: u16, max: u16) -> u16 {
        let mut rng = rand::thread_rng();
        rng.gen_range(min..max - 1)
    }


    fn test(&mut self, i: u16) {
        // if philosopher i is hungry and both neighbours are not eating then eat
        println!("philosopher {:?} thread {:?} test enter", i, thread::current().id());
        // i: philosopher number, from 0 to N-
        //let mut _output_mtx = self.output_mtx.lock().unwrap();
        if self.seats[i as usize] == State::HUNGRY &&
            self.seats[self.left(i) as usize] != State::EATING &&
            self.seats[self.right(i) as usize] != State::EATING 
        {
            self.seats[i as usize] = State::EATING;

            println!("SYD {:?} is EATING!!", i);
           
            let (mutex, condvar) = &self.both_forks_available[i as usize];
            println!("philosopher {:?} thread {:?} test waiting on mutex", i, thread::current().id());
            let mut started = mutex.lock().unwrap();
            println!("philosopher {:?} thread started {:?} test got mutex started {:?}", i, *started, thread::current().id());
            *started = true;

            drop(started);
            
            condvar.notify_one();

            println!("philosopher {:?} thread {:?} notified condition var test() leaving", i, thread::current().id());
          
            //drop(self.both_forks_available_permit[i as usize].as_ref()); // forks are no longer needed for this eat session
        }
        println!("philosopher {:?} thread {:?} test leave", i, thread::current().id());
    }


    fn think(&self, i: u16) {
        println!("philosopher {:?} thread {:?} think enter", i, thread::current().id());
        let duration = self.my_rand(400, 800);
        {
            let mut _output_mtx = self.output_mtx.lock().unwrap();
            // critical section for uninterrupted print
            println!("{:?} is thinking {:?} ms", i, duration);
        }
        thread::sleep(time::Duration::from_millis(duration.into()));
        println!("philosopher {:?} thread {:?} think leave", i, thread::current().id());
    }

    fn take_forks(&mut self, i: u16) {
            {
                //let mut _output_mtx = self.output_mtx.lock().unwrap(); // critical section for uninterrupted print
                println!("\t\tPhiliosoper {:?} is enter take_forks", i);
            }
            let _critical_region_mtx = Box::new(self.critical_region_mtx.lock().unwrap()).as_mut();  // enter critical region
            self.seats[i as usize] = State::HUNGRY;  // record fact that philosopher i is State::HUNGRY
            {
                let mut _output_mtx = self.output_mtx.lock().unwrap(); // critical section for uninterrupted print
                println!("\t\tphilosopher {:?} is State::HUNGRY", i);
            }
            self.test(i);                        // try to acquire (a permit for) 2 forks  

                                   // exit critical region
        //self.both_forks_available_permit.push(Some(self.both_forks_available[i as usize].acquire().await.unwrap()));  // block if forks were not acquired
        //let _permit = self.both_forks_available[i as usize].acquire().await; 
        //println!("SYD permit is {:?}", _permit);

        let (lock, cvar) = &self.both_forks_available[i as usize];
        let mut started = lock.lock().unwrap();
        while !*started {
            println!("philosopher {:?} thread {:?} waiting for condition varable", i, thread::current().id());
            started = cvar.wait(started).unwrap();
        }
        println!("philosopher {:?} thread {:?} take_forks got condition variable started {:?}", i, thread::current().id(), *started);
        *started = false;
        drop(started);
        println!("philosopher {:?} thread {:?} take_forks leave", i, thread::current().id());

        //self.both_forks_available[i as usize].lock();
    }

    fn eat(&self, i: u16) {
        println!("philosopher {:?} thread {:?} eat enter", i, thread::current().id());
        let duration = self.my_rand(400, 800);
        {
            let mut _output_mtx = self.output_mtx.lock().unwrap(); // critical section for uninterrupted print
            println!("\t\t\t\tphilosopher {:?} is eating duration {:?} ms", i ,duration);
        }
        thread::sleep(time::Duration::from_millis(duration.into()));
        println!("philosopher {:?} thread {:?} eat leave", i, thread::current().id());
    }

    fn put_forks(&mut self, i: u16)  { 
        println!("philosopher {:?} thread {:?} put_forks enter", i, thread::current().id()); 
        let _critical_region_mtx = Box::new(self.critical_region_mtx.lock().unwrap()).as_mut();  // enter critical region
        self.seats[i as usize] = State::THINKING;  // philosopher has finished State::EATING
        let left = self.left(i);
        let right = self.right(i);
        self.test(left);               // see if left neighbor can now eat
        self.test(right);              // see if right neighbor can now eat
                                               // exit critical region by exiting the function
        println!("philosopher {:?} thread {:?} put_forks leave", i, thread::current().id()); 
    }

    async fn philosopher(&mut self, i: u16) {  
        println!("SYD philospoher {:?} thread {:?}", i, thread::current().id());
        loop {                         // repeat forever
            self.think(i);             // philosopher is State::THINKING
            self.take_forks(i);        // acquire two forks or block
            self.eat(i);               // yum-yum, spaghetti
            self.put_forks(i);         // put both forks back on table and check if neighbours can eat
        };
    }
}

/* 
State state[N];  // array to keep track of everyone's both_forks_available state

std::mutex critical_region_mtx;  // mutual exclusion for critical regions for 
// (picking up and putting down the forks)
std::mutex output_mtx;  // for synchronized cout (printing THINKING/HUNGRY/EATING status)
*/

// array of binary semaphors, one semaphore per philosopher.
// Acquired semaphore means philosopher i has acquired (blocked) two forks
/* 
std::binary_semaphore both_forks_available[N]
{
    std::binary_semaphore{0}, std::binary_semaphore{0},
    std::binary_semaphore{0}, std::binary_semaphore{0},
    std::binary_semaphore{0}
};
*/

/* 
int main() {
    std::cout << "dp_14\n";

    std::jthread t0([&] { philosopher(0); }); // [&] means every variable outside the ensuing lambda 
    std::jthread t1([&] { philosopher(1); }); // is captured by reference
    std::jthread t2([&] { philosopher(2); });
    std::jthread t3([&] { philosopher(3); });
    std::jthread t4([&] { philosopher(4); });
}
*/

async fn launch_threads_wrapper() {
    let foo = launch_threads();
    foo.await;
}

async fn launch_threads() {
    let my_table = DiningPhilosphersTableBuilder::new(N);
    //let mut threads: Vec<threads::JoinHandle<_>> = Vec::<threads::JoinHandle<_>>::new();
    let table = Arc::new(Mutex::new(my_table));
    //let futs = Vec::<Box::<dyn Future>>::new();
    //let threads = Vec::<u16>::new();
    for i in 0..N {
        println!("SYD top of loop {:?}", i);
        let data = Arc::clone(&table);
        //threads.push(thread::spawn(move || {
            println!("SYD thread {:?}", i);
            {
                let mut shared = data.lock().unwrap();
                //threads.push(shared.philosopher(i));
                let foo = shared.philosopher(i);
                foo.await;
            }
       
            // The shared state can only be accessed once the lock is held.
            // Our non-atomic increment is safe because we're the only thread
            // which can access the shared state when the lock is held.
            //
            // We unwrap() the return value to assert that we are not expecting
            // threads to ever fail while holding the lock.
            //let mut data = data.lock().unwrap();
            
            // the lock is unlocked here when `data` goes out of scope.
        //}));
    }

    //threads
    //futures::join!(futs);

}

use futures::executor::block_on;

fn main() {
    block_on(launch_threads_wrapper());

    println!("back from threads");

    loop {}
    
    /* 
    for thread in threads {
        println!("SYD thread join");
        thread.join();
    }
    println!("SYD main exit");
    */

}
