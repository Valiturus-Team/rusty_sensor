# RUSTY SENSOR

This repository is the firmware for the valirutus sensor.

## Dev notes

Code that is hardware agnostic should be split into sub crates where it can be unit tested, i.e. the app_state sub crate.
The root project, should contain the hardware specific code and contains the entry point of the application. 

## building and flashing firmware

Need to install espflash from cargo
cargo install espflash

#### flashing and monitoring the sensor


espflash --monitor /dev/ttyUSB0 --partition-table partitions_two_ota.csv /home/lance/rusty-sensor

you may need to erase flash if an ota partition is active

#### Creating update file

espflash save-image esp32 rusty-sensor esp.bin

## Messaging Diagram
```mermaid
  sequenceDiagram

  
    APP->>SENSOR: subscribe to byte_out_characterisitic
    App->>SENSOR: write to byte_in_characteristic
    
    SENSOR->>APP: hey I have a message

    loop Send message bytes
      APP->>SENSOR: SEND DATA CHUNK
      SENSOR->>SENSOR: Write data chunk to other ota partition.
      SENSOR->>APP: DATA ACK
    end


```

## OTA Update mechanisim 
(happy path only for now, todo: add in error paths)
```mermaid  
  sequenceDiagram
      APP->>SENSOR: OTA CONTROL REQUEST
      SENSOR->>SENSOR: set packet size, set updating == true.
      SENSOR->>APP: OTA CONTROL REQUEST ACK

      loop Send .bin image / update file
        APP->>SENSOR: SEND DATA CHUNK
        SENSOR->>SENSOR: Write data chunk to other ota partition.
        SENSOR->>APP: DATA ACK
      end

      APP->>SENSOR: OTA CONTROL DONE (Sent everything)
      SENSOR->>SENSOR: Stop writing to partition, clear variables.
      SENSOR->>APP: OTA CONTROL DONE ACK

      APP->>SENSOR: APPLY UPDATE 
      SENSOR->>APP: APPLY UPDATE ACK

      SENSOR->>SENSOR: Sensor Reboot

      opt Can Connect after reboot
        APP->>SENSOR: Can I still talk to you
        SENSOR->>APP: Yessir
        SENSOR->SENSOR: Verify New Image! ()
      end
      opt Cannot Connect After X seconds/attempts
        APP->>SENSOR: Can I still talk to you
        APP->>SENSOR: Can I still talk to you
        APP->>SENSOR: Can I still talk to you
        APP->>SENSOR: Can I still talk to you
        
        SENSOR->SENSOR: Rollback to previous image
      end

```



