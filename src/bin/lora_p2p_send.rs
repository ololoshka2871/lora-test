//! This example runs on the STM32 LoRa Discovery board, which has a builtin Semtech Sx1276 radio.
//! It demonstrates LORA P2P send functionality.
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::exti::{Channel, ExtiInput};
use embassy_stm32::gpio::{Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::spi;
use embassy_stm32::time::khz;
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::iv::GenericSx127xInterfaceVariant;
use lora_phy::sx127x::{Sx127x, Sx1276};
use lora_phy::LoRa;
use lora_phy::{mod_params::*, sx127x};
use {defmt_rtt as _, panic_probe as _};

const LORA_FREQUENCY_IN_HZ: u32 = 434_000_000; // warning: set this appropriately for the region

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = true;
    config.rcc.pll_src = embassy_stm32::rcc::PllSource::HSI;
    let p = embassy_stm32::init(config);

    let nss = Output::new(p.PA4.degrade(), Level::High, Speed::Low);
    let reset = Output::new(p.PA3.degrade(), Level::High, Speed::Low);
    let irq_pin = Input::new(p.PA9.degrade(), Pull::Up);
    let irq = ExtiInput::new(irq_pin, p.EXTI9.degrade());

    let mut spi_config = spi::Config::default();
    spi_config.frequency = khz(200);
    let spi = spi::Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6, p.DMA2_CH3, p.DMA2_CH2, spi_config,
    );
    let spi = ExclusiveDevice::new(spi, nss, Delay);

    let config = sx127x::Config {
        chip: Sx1276,
        tcxo_used: false,
        rx_boost: false,
        tx_boost: true,
    };
    let iv = GenericSx127xInterfaceVariant::new(reset, irq, None, None).unwrap();
    let mut lora = LoRa::new(Sx127x::new(spi, iv, config), false, Delay)
        .await
        .unwrap();

    let mdltn_params = {
        match lora.create_modulation_params(
            SpreadingFactor::_12,
            Bandwidth::_10KHz,
            CodingRate::_4_8,
            LORA_FREQUENCY_IN_HZ,
        ) {
            Ok(mp) => mp,
            Err(err) => {
                info!("Radio error = {}", err);
                return;
            }
        }
    };

    let mut tx_pkt_params = {
        match lora.create_tx_packet_params(4, false, true, false, &mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                info!("Radio error = {}", err);
                return;
            }
        }
    };

    let buffer = [0x01u8, 0x02u8, 0x03u8];

    match lora
        .prepare_for_tx(&mdltn_params, &mut tx_pkt_params, 20, &buffer)
        .await
    {
        Ok(()) => {}
        Err(err) => {
            info!("Radio error = {}", err);
            return;
        }
    };

    match lora.tx().await {
        Ok(()) => {
            info!("TX DONE");
        }
        Err(err) => {
            info!("Radio error = {}", err);
            return;
        }
    };

    match lora.sleep(false).await {
        Ok(()) => info!("Sleep successful"),
        Err(err) => info!("Sleep unsuccessful = {}", err),
    }
}
