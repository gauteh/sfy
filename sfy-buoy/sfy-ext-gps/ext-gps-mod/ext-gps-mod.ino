

# ifdef GPS
# include "gps.h"
# endif

Uart sfy{1, 40, 39};  // Serial to SFY

void setup()
{
  Serial.begin(115200);
  Serial.println(F("SFY-RTK bridge"));

# ifdef GPS
  setup_gps();
# endif

  sfy.begin(400000);
  sfy.println("{'status':'startup'}");
}

//=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=

void loop()
{
# ifdef GPS
  loop_gps();
# endif
}
