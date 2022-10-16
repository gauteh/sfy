import {Component, createRef} from 'react';
import { MapContainer, TileLayer, useMap, Marker, Popup, Tooltip } from 'react-leaflet';
// import L from 'leaflet';

import {Buoy, OmbBuoy} from 'models';

import './BuoyIndex.scss';
import 'leaflet/dist/leaflet.css';
// import marker_me from './marker-me.png';
// import marker_icon from 'leaflet/dist/images/marker-icon-2x.png';
// import marker_shadow from 'leaflet/dist/images/marker-shadow.png';

const MAPBOX_TOKEN: string = 'pk.eyJ1IjoiZ2F1dGVoIiwiYSI6ImNreWZ2MWd4NjBxNnoyb3M4eWRqNjlmMGMifQ.m-5Q9BBf2yQxp1fGStxYRg';

interface Props {
  buoys: Array<Buoy | OmbBuoy>;
}

interface State {
  myself: [] | undefined;
}

export class BuoyMap
  extends Component<Props, State>
{

  public state = { myself: undefined };
  map: any = undefined

  constructor(props: Props) {
    super(props);

    // this.map = useMap();

    if (navigator.geolocation) {
      navigator.geolocation.watchPosition(this.updateMyPosition);
    }
  }

  public updateMyPosition = (position: any) => {
    console.log("Got new position:" + position);
    // if (this.map != undefined) {
    //   if (this.myselfMarker === undefined) {
    //     const icon = L.icon({
    //       iconUrl: marker_me,

    //       iconSize:     [30, 38], // size of the icon
    //       // shadowSize:   [50, 64], // size of the shadow
    //       iconAnchor:   [15, 38], // point of the icon which will correspond to marker's location
    //       // shadowAnchor: [4, 62],  // the same for the shadow
    //       // popupAnchor:  [-3, -76] // point from which the popup should open relative to the iconAnchor
    //     });

    //     this.myselfMarker = L.marker([position.coords.latitude, position.coords.longitude], {icon: icon}).addTo(this.map);
    //     this.myselfMarker.bindTooltip('Your position');
    //   } else {
    //     this.myselfMarker.setLatLng([position.coords.latitude, position.coords.longitude]);
    //   }
    // }
  }

  public focus = (buoy: any) => {
    console.log("Focusing: " + buoy);

    this.map.flyTo([buoy.any_lat(), buoy.any_lon()], 11);
  }

  public render() {
    return (
      <MapContainer className="container-fluid" center={[51.505, -0.09]} zoom={13} scrollWheelZoom={false}>

        <TileLayer
          attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
          url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
        />

        <TileLayer url='https://t1.openseamap.org/seamark/{z}/{x}/{y}.png' />

        <Marker position={[51.505, -0.09]}>
          <Popup>
            A pretty CSS3 popup. <br /> Easily customizable.
          </Popup>
        </Marker>
      </MapContainer>
    );
  }
}


      // <div id="map-container" className="container-fluid">
      //   <div id="map"></div>
      //   <MapContainer center={[60.11304848114283, 2.3882482939071434]} zoom={5}>
      //     <TileLayer url='https://t1.openseamap.org/seamark/{z}/{x}/{y}.png' />
      //   </MapContainer>
      // </div>

          // <TileLayer
          //   attribution='Map data &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors Imagery © <a href="https://www.mapbox.com/">Mapbox</a>'
          //   url='https://api.mapbox.com/styles/v1/{id}/tiles/{z}/{x}/{y}?access_token={accessToken}'
          //   id='mapbox/light-v10'
          //   accessToken={MAPBOX_TOKEN} />

          // {
          // this.props.buoys.filter((buoy) => buoy.any_lat() != undefined).map((buoy) => {
          //     <Marker position={[buoy.any_lat() as number, buoy.any_lon() as number]}>
          //       <Tooltip>
          //         {`${buoy.sn} (${buoy.dev})`}
          //       </Tooltip>
          //     </Marker>
          //   } }

