#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
};

use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::hprintln;

use stm32f4xx_hal::{
    gpio::{AnyPin, Edge, Output, PushPull},
    hal::digital::OutputPin,
    pac::{self, interrupt},
    prelude::*,
    rcc::Config,
};

#[panic_handler]
fn panic_halt(_info: &PanicInfo) -> ! {
    loop {}
}

const CYCLE_DELAY: u32 = 12_500_000;
const NUM_FLOORS: usize = 5;

#[entry]
fn main() -> ! {
    let mut dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();
    let mut rcc = dp.RCC.constrain().freeze(Config::default());

    // configure clock
    let mut systick = cp.SYST;
    systick.enable_interrupt();
    systick.set_clock_source(SystClkSource::Core);
    systick.set_reload(CYCLE_DELAY);
    systick.clear_current();
    systick.enable_counter();

    // input pins
    let gpiob = dp.GPIOB.split(&mut rcc);
    let mut syscfg = dp.SYSCFG.constrain(&mut rcc);

    // pb1
    let mut button1 = gpiob.pb1.into_pull_up_input();
    button1.make_interrupt_source(&mut syscfg);
    button1.trigger_on_edge(&mut dp.EXTI, Edge::Falling);
    button1.enable_interrupt(&mut dp.EXTI);
    // pb2
    let mut button2 = gpiob.pb2.into_pull_up_input();
    button2.make_interrupt_source(&mut syscfg);
    button2.trigger_on_edge(&mut dp.EXTI, Edge::Falling);
    button2.enable_interrupt(&mut dp.EXTI);
    // pb3
    let mut button3 = gpiob.pb3.into_pull_up_input();
    button3.make_interrupt_source(&mut syscfg);
    button3.trigger_on_edge(&mut dp.EXTI, Edge::Falling);
    button3.enable_interrupt(&mut dp.EXTI);
    // pb4
    let mut button4 = gpiob.pb4.into_pull_up_input();
    button4.make_interrupt_source(&mut syscfg);
    button4.trigger_on_edge(&mut dp.EXTI, Edge::Falling);
    button4.enable_interrupt(&mut dp.EXTI);
    // pb5
    let mut button5 = gpiob.pb5.into_pull_up_input();
    button5.make_interrupt_source(&mut syscfg);
    button5.trigger_on_edge(&mut dp.EXTI, Edge::Falling);
    button5.enable_interrupt(&mut dp.EXTI);

    unsafe {
        pac::NVIC::unmask(pac::Interrupt::EXTI1);
        pac::NVIC::unmask(pac::Interrupt::EXTI2);
        pac::NVIC::unmask(pac::Interrupt::EXTI3);
        pac::NVIC::unmask(pac::Interrupt::EXTI4);
        pac::NVIC::unmask(pac::Interrupt::EXTI9_5);
    }

    // output pins
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut elevator_pos_lights: [AnyPin<Output<PushPull>>; NUM_FLOORS] = [
        // pa12
        gpioa.pa12.into_push_pull_output().erase(),
        // pa5
        gpioa.pa5.into_push_pull_output().erase(),
        // pa7
        gpioa.pa7.into_push_pull_output().erase(),
        // pa8
        gpioa.pa8.into_push_pull_output().erase(),
        // pa1
        gpioa.pa1.into_push_pull_output().erase(),
    ];
    let mut elevator_call_lights: [AnyPin<Output<PushPull>>; NUM_FLOORS] = [
        // pa11
        gpioa.pa11.into_push_pull_output().erase(),
        // pa6
        gpioa.pa6.into_push_pull_output().erase(),
        // pa9
        gpioa.pa9.into_push_pull_output().erase(),
        // pa10
        gpioa.pa10.into_push_pull_output().erase(),
        // pa4
        gpioa.pa4.into_push_pull_output().erase(),
    ];
    loop {
        while !DO_TICK.load(Ordering::Acquire) {}
        DO_TICK.store(false, Ordering::Release);
        elevator_pos_lights.iter_mut().for_each(|light| {
            light.set_low();
        });
        let ecm = ELEVATOR_CALL_MASK.swap(0, Ordering::Acquire);
        if ecm != 0 {
            if ecm & MASK_PB1 != 0 {
                elevator_call_lights[0].set_high();
            }
            if ecm & MASK_PB2 != 0 {
                elevator_call_lights[1].set_high();
            }
            if ecm & MASK_PB3 != 0 {
                elevator_call_lights[2].set_high();
            }
            if ecm & MASK_PB4 != 0 {
                elevator_call_lights[3].set_high();
            }
            if ecm & MASK_PB5 != 0 {
                elevator_call_lights[4].set_high();
            }
        }
        let idx = ELEVATOR_POS_IDX.load(Ordering::Acquire);
        elevator_pos_lights[idx].set_high();
        elevator_call_lights[idx].set_low();
    }
}

static DO_TICK: AtomicBool = AtomicBool::new(true);
static ELEVATOR_POS_IDX: AtomicUsize = AtomicUsize::new(0);
static ELEVATOR_CALL_MASK: AtomicU8 = AtomicU8::new(0);

#[exception]
fn SysTick() {
    hprintln!("Tick");
    DO_TICK.store(true, Ordering::Release);
    let mut new_val = ELEVATOR_POS_IDX.load(Ordering::Acquire);
    new_val = (new_val + 1) % NUM_FLOORS;
    ELEVATOR_POS_IDX.store(new_val, Ordering::Release);
}

const MASK_PB1: u8 = 0b10;
const MASK_PB2: u8 = 0b100;
const MASK_PB3: u8 = 0b1000;
const MASK_PB4: u8 = 0b10000;
const MASK_PB5: u8 = 0b100000;

#[interrupt]
fn EXTI1() {
    hprintln!("Triggered ISR 1");
    DO_TICK.store(true, Ordering::Release);
    let mut val = ELEVATOR_CALL_MASK.load(Ordering::Acquire);
    let mask = MASK_PB1;
    val |= mask;
    ELEVATOR_CALL_MASK.store(val, Ordering::Release);
    unsafe {
        (*pac::EXTI::ptr()).pr().write(|w| w.bits(mask as u32));
    }
}

#[interrupt]
fn EXTI2() {
    hprintln!("Triggered ISR 2");
    DO_TICK.store(true, Ordering::Release);
    let mut val = ELEVATOR_CALL_MASK.load(Ordering::Acquire);
    let mask = MASK_PB2;
    val |= mask;
    ELEVATOR_CALL_MASK.store(val, Ordering::Release);
    unsafe {
        (*pac::EXTI::ptr()).pr().write(|w| w.bits(mask as u32));
    }
}

#[interrupt]
fn EXTI3() {
    hprintln!("Triggered ISR 3");
    DO_TICK.store(true, Ordering::Release);
    let mut val = ELEVATOR_CALL_MASK.load(Ordering::Acquire);
    let mask = MASK_PB3;
    val |= mask;
    ELEVATOR_CALL_MASK.store(val, Ordering::Release);
    unsafe {
        (*pac::EXTI::ptr()).pr().write(|w| w.bits(mask as u32));
    }
}

#[interrupt]
fn EXTI4() {
    hprintln!("Triggered ISR 4");
    DO_TICK.store(true, Ordering::Release);
    let mut val = ELEVATOR_CALL_MASK.load(Ordering::Acquire);
    let mask = MASK_PB4;
    val |= mask;
    ELEVATOR_CALL_MASK.store(val, Ordering::Release);
    unsafe {
        (*pac::EXTI::ptr()).pr().write(|w| w.bits(mask as u32));
    }
}

#[interrupt]
fn EXTI9_5() {
    hprintln!("Triggered ISR 5");
    DO_TICK.store(true, Ordering::Release);
    let mut val = ELEVATOR_CALL_MASK.load(Ordering::Acquire);
    let mask = MASK_PB5;
    val |= mask;
    ELEVATOR_CALL_MASK.store(val, Ordering::Release);
    unsafe {
        (*pac::EXTI::ptr()).pr().write(|w| w.bits(mask as u32));
    }
}
