HOST=http://localhost:3000
SFY_AUTH_TOKEN=thicaige6oagh3izohvie5Piech9cu5eiweuk5eumoh4aeshooXaghefeme2aizohnohtoowaequ1Ieghuoc8aboj8owuiPai8johthaif8chie1tahgh9in

set -ep

curl -f -v -X POST "${HOST}/buoy" \
  -H "SFY_AUTH_TOKEN: ${SFY_AUTH_TOKEN}" \
  --data-binary "@events/1647870799330-1876870b-4708-4366-8db5-68f872cc4e6d_axl.qo.json"

curl -f -v -X POST "${HOST}/buoy" \
  -H "SFY_AUTH_TOKEN: ${SFY_AUTH_TOKEN}" \
  --data-binary "@events/1653994017660-ae50c1e9-0800-4fd9-9cb6-cdd6a6d08eb3_storage.db.json"

curl -f -v -X POST "${HOST}/buoy" \
  -H "SFY_AUTH_TOKEN: ${SFY_AUTH_TOKEN}" \
  --data-binary "@events/sensor.db_01.json"

curl -f -v -X POST "${HOST}/buoy/omb" \
  -H "SFY_AUTH_TOKEN: ${SFY_AUTH_TOKEN}" \
  --data-binary "@events/01-omb.json"
