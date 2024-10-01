# include "ArduinoJson.h"
# include "gps.h"

// Serial to SFY
// TX1: ~7 / 42
// RX1: ~8 / 38
// ..
// TX1: ~9 / 39
// RX1: ~10 / 40
Uart sfy{1, 40, 39};

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
  static int i = 0;

  loop_gps();
  Serial.print(".");
  delay(100);

  JsonDocument doc;
  doc["loop"] = i;
  serializeJson(doc, sfy);

  i++;
}

void main_pps() {
  Serial.println("PPS!");
  pps();
}
