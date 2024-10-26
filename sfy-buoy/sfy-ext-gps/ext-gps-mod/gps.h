# define GPS
# ifndef GPS_H
# define GPS_H
# include <SparkFun_u-blox_GNSS_v3.h> //http://librarymanager/All#SparkFun_u-blox_GNSS_v3
# include <vector>

#define OK(ok) (ok ? F("  ->  OK") : F("  ->  ERROR!")) // Convert uint8_t into OK/ERROR

const uint16_t PPS_PIN = 11; // ~AD2
                             //

const uint32_t SOL_FREQ = 5;
const uint32_t TP_FREQ  = 5;

void setup_gps();
void loop_gps();
void pps();

# endif
