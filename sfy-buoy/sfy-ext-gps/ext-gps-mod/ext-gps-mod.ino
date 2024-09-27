# include "ArduinoJson.h"
# include "gps.h"

// Serial to SFY
// TX1: ~7 / 42
// RX1: ~8 / 38
Uart sfy{1, 38, 42};

void setup()
{
  Serial.begin(115200);
  Serial.println(F("SFY-RTK bridge"));

  setup_gps();

  sfy.begin(400000);

  JsonDocument doc;
  doc["status"] = "startup";
  serializeJson(doc, sfy);

  pinMode(PPS_PIN, INPUT_PULLUP);
  attachInterrupt(digitalPinToInterrupt(PPS_PIN), main_pps, RISING);
}

//=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=

void loop()
{
  loop_gps();
  Serial.print(".");
  delay(50);
}

void main_pps() {
  Serial.println("PPS!");
  pps();
}
