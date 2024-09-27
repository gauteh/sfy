# ifndef GPS_H
# define GPS_H
# include <SparkFun_u-blox_GNSS_v3.h> //http://librarymanager/All#SparkFun_u-blox_GNSS_v3
#include <vector>
                                      //
//const uint32_t myLBandFreq = 1556290000; // Uncomment this line to use the US SPARTN 1.8 service
//const uint32_t myLBandFreq = 1545260000; // Uncomment this line to use the EU SPARTN 1.8 service

#define OK(ok) (ok ? F("  ->  OK") : F("  ->  ERROR!")) // Convert uint8_t into OK/ERROR
                                                        //

static uint32_t pps_ts = 0;

typedef struct GpsM_t {
    uint64_t cputime = 0;
    int64_t  ppsdiff = 0;
    uint64_t gpstime = 0;
    double lat = 0.0;
} GpsM;

void setup_gps();
void loop_gps();
void pps();

# endif
