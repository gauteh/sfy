# include "ArduinoJson.h"

# ifdef GPS
# include "gps.h"
# endif

// Serial to SFY
// TX1: ~7 / 42
// RX1: ~8 / 38
Uart sfy{1, 38, 42};

void setup()
{
  Serial.begin(115200);
  Serial.println(F("SFY-RTK bridge"));

# ifdef GPS
  setup_gps();
# endif

  sfy.begin(400000);

  JsonDocument doc;
  doc["status"] = "startup";
  serializeJson(doc, sfy);
}

//=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=

void loop()
{
# ifdef GPS
  loop_gps();
# endif
}
