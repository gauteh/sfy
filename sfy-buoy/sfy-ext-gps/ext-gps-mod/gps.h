# define GPS
# ifndef GPS_H
# define GPS_H
# include <SparkFun_u-blox_GNSS_v3.h> //http://librarymanager/All#SparkFun_u-blox_GNSS_v3
# include <vector>

#define OK(ok) (ok ? F("  ->  OK") : F("  ->  ERROR!")) // Convert uint8_t into OK/ERROR

const uint16_t PPS_PIN = 11; // ~AD2

static uint64_t pps_ts = 0;

typedef struct GpsM_t {
    uint64_t cputime = 0;
    int64_t  ppsdiff = 0;
    uint64_t gpstime = 0;
    double lat = 0.0;
    double lon = 0.0;
    double height = 0.0;
} GpsM;

void setup_gps();
void loop_gps();
void pps();

# endif
