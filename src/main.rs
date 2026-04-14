use tokio::net::UdpSocket;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use std::time::Duration;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. MQTT-asetukset
    let mut mqttoptions = MqttOptions::new("meshcom_bridge", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    
    // Tilaa aihe, josta viestit lähetetään MeshComiin
    client.subscribe("meshcom/tx", QoS::AtMostOnce).await?;

    // 2. UDP-asetukset
    // Arc (Atomic Reference Counted) tarvitaan, jotta samaa socketia voi käyttää kahdessa taskissa
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:1799").await?);
    let socket_tx = socket.clone();
    
    // Lilygon osoite (muuta jos Lilygon IP vaihtuu)
    let lilygo_addr = "10.42.0.2:1990"; 

    println!("MeshCom silta käynnistetty...");

    // TEHTÄVÄ 1: MQTT -> MeshCom (Lähetys)
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(notification) => {
                    if let Event::Incoming(Packet::Publish(p)) = notification {
                        let msg = String::from_utf8_lossy(&p.payload);
                        println!("MQTT -> MeshCom: {}", msg);
                        
                        // Lähetetään viesti UDP:llä Lilygon porttiin 1990
                        if let Err(e) = socket_tx.send_to(msg.as_bytes(), lilygo_addr).await {
                            println!("UDP lähetysvirhe: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("MQTT-virhe: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // TEHTÄVÄ 2: MeshCom -> MQTT (Vastaanotto)
    let mut buf = [0u8; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let message = String::from_utf8_lossy(&buf[..len]);
        
        println!("MeshCom -> MQTT (lähde {}): {}", addr, message);

        // Julkaistaan vastaanotettu viesti
        client.publish("meshcom/rx", QoS::AtLeastOnce, false, message.to_string()).await?;
    }
}
