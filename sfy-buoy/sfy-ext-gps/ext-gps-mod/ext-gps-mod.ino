# include "ArduinoJson.h"
# include "gps.h"

void setup()
{
  Serial.begin(115200);
  Serial.println(F("SFY-RTK bridge"));

  setup_gps();

  // JsonDocument doc;
  // doc["status"] = "startup";
  // serializeJson(doc, sfy);

  pinMode(PPS_PIN, INPUT_PULLUP);
  attachInterrupt(digitalPinToInterrupt(PPS_PIN), main_pps, FALLING);
}

//=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=

void loop()
{
  loop_gps();
}

void main_pps() {
  Serial.println("PPS!");
  pps();
}
