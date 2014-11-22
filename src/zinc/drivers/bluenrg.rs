// Zinc, the bare metal stack for rust.
// Copyright 2014 Dzmitry "kvark" Malyshau <kvarkus@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! BlueNRG low-level SPI communication.
// http://www.st.com/st-web-ui/static/active/en/resource/technical/document/user_manual/DM00114498.pdf

use core::result::{Result, Ok, Err};
use core::slice::SlicePrelude;

use hal::pin::Gpio;
use hal::spi::Spi;

#[repr(u8)]
enum SpiControl {
  SpiWrite = 0x0A,
  SpiRead = 0x0B,
}

/// Spi error codes.
#[repr(u8)]
pub enum SpiError {
  /// Device is sleeping.
  SpiSleeping,
  /// Status is unlnown.
  SpiUnknown(u8),
  /// Given buffer is too large.
  SpiBufferSize(u16),
}

/// BlueNRG driver.
pub struct BlueNrg<G, S> {
  active: G,
  //input: G,
  //output: G,
  spi: S,
}

impl<G: Gpio, S: Spi> BlueNrg<G, S> {
  /// Create a new BlueNRG driver instance.
  pub fn new(active: G, spi: S) -> BlueNrg<G, S> {
    active.set_high();
    BlueNrg {
      active: active,
      spi: spi,
    }
  }

  /// Check device status and return the maximum write/read data sizes.
  pub fn check(&self) -> Result<(u16, u16), SpiError> {
    self.active.set_low();
    // A return frame is 5 bytes, where the 1st byte is a status,
    // then 2 bytes of the maximum write buffer size,
    // and then 2 bytes for the maximum read buffer.
    let status = self.spi.transfer(SpiRead as u8);
    let w0 = self.spi.transfer(0);
    let w1 = self.spi.transfer(0);
    let r0 = self.spi.transfer(0);
    let r1 = self.spi.transfer(0);
    self.active.set_high();

    match status {
      0x02 => Ok((
        (w0 as u16 << 8) | (w1 as u16), // write buffer size
        (r0 as u16 << 8) | (r1 as u16), // read buffer size
      )),
      0x00 | 0xFF => Err(SpiSleeping),
      other => Err(SpiUnknown(other)),
    }
  }

  /// Poll the device until it wakes up.
  pub fn wakeup(&self, mut num_tries: u32) -> Result<(u16, u16), SpiError> {
    loop {
      match self.check() {
        Err(SpiSleeping) if num_tries > 0 => {
          num_tries -= 1;
        },
        other => return other,
      }
    }
  }

  /// Receive data into the given buffer.
  pub fn receive(&self, buf: &mut [u8]) -> Result<(), SpiError> {
    self.active.set_low();
    let status = self.spi.transfer(SpiRead as u8);
    self.spi.transfer(0);
    self.spi.transfer(0);
    let r0 = self.spi.transfer(0);
    let r1 = self.spi.transfer(0);
    let size = (r0 as u16 << 8) | (r1 as u16);
    if status != 0x02 {
      self.active.set_high();
      Err(SpiUnknown(status))
    }else if size < buf.len() as u16 {
      self.active.set_high();
      Err(SpiBufferSize(size))
    }else {
      for b in buf.iter_mut() {
        *b = self.spi.transfer(0);
      }
      self.active.set_high();
      Ok(())
    }
  }

  /// Send data from the given buffer.
  pub fn send(&self, buf: &[u8]) -> Result<(), SpiError> {
    self.active.set_low();
    let status = self.spi.transfer(SpiWrite as u8);
    let w0 = self.spi.transfer(0);
    let w1 = self.spi.transfer(0);
    self.spi.transfer(0);
    self.spi.transfer(0);
    let size = (w0 as u16 << 8) | (w1 as u16);
    if status != 0x02 {
      self.active.set_high();
      Err(SpiUnknown(status))
    }else if size < buf.len() as u16 {
      self.active.set_high();
      Err(SpiBufferSize(size))
    }else {
      for b in buf.iter() {
        self.spi.transfer(*b);
      }
      self.active.set_high();
      Ok(())
    }
  }
}
