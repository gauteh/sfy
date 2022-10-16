import { Component } from 'react';
import { MapContainer, TileLayer, Marker, Tooltip, useMap } from 'react-leaflet';
import L from 'leaflet';
import { Buoy, OmbBuoy } from 'models';

import './BuoyIndex.scss';
import 'leaflet/dist/leaflet.css';
import icon_me from './marker-me.png';
import icon from 'leaflet/dist/images/marker-icon.png';
import iconShadow from 'leaflet/dist/images/marker-shadow.png';

const DefaultIcon = L.icon({
  iconUrl: icon,
  shadowUrl: iconShadow,
  iconSize: [30, 40], // size of the icon
  iconAnchor: [15, 40], // point of the icon which will correspond to marker's location
});

L.Marker.prototype.options.icon = DefaultIcon;

const myselfIcon = L.icon({
  iconUrl: icon_me,

  iconSize: [30, 38], // size of the icon
  // shadowSize:   [50, 64], // size of the shadow
  iconAnchor: [15, 38], // point of the icon which will correspond to marker's location
  // shadowAnchor: [4, 62],  // the same for the shadow
  // popupAnchor:  [-3, -76] // point from which the popup should open relative to the iconAnchor
});

const MAPBOX_TOKEN: string = 'pk.eyJ1IjoiZ2F1dGVoIiwiYSI6ImNreWZ2MWd4NjBxNnoyb3M4eWRqNjlmMGMifQ.m-5Q9BBf2yQxp1fGStxYRg';

interface Props {
  buoys: Array<Buoy | OmbBuoy>;
}

interface State {
  myself?: Array<number>;
}

export class BuoyMap
  extends Component<Props, State>
{

  public state: State = { myself: undefined };
  map: any = undefined

  constructor(props: Props) {
    super(props);

    if (navigator.geolocation) {
      navigator.geolocation.watchPosition(this.updateMyPosition);
    }
  }

  public updateMyPosition = (position: any) => {
    console.log("Got new position:" + position);
    this.state.myself = [position.coords.latitude, position.coords.longitude];
  }

  public focus = (buoy: any) => {
    console.log("Focusing: " + JSON.stringify(buoy));

    this.map.flyTo([buoy.any_lat(), buoy.any_lon()], 11);
  }

  public BuoyMarker = (buoy: any) => {
    return (
      <Marker key={buoy.dev} position={[buoy.any_lat() as number, buoy.any_lon() as number]}>
        <Tooltip>
          {buoy.sn} ({buoy.dev})
        </Tooltip>
      </Marker>
    )
  }

  public MapController = () => {
    this.map = useMap();

    return (<></>)
  }

  public render() {
    // <TileLayer
    //   attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
    //   url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
    // />

    return (
      <MapContainer className="container-fluid" center={[60.11304848114283, 2.3882482939071434]} zoom={5}>
        <TileLayer
          attribution='Map data &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors Imagery © <a href="https://www.mapbox.com/">Mapbox</a>'
          url='https://api.mapbox.com/styles/v1/{id}/tiles/{z}/{x}/{y}?access_token={accessToken}'
          id='mapbox/light-v10'
          accessToken={MAPBOX_TOKEN} />

        <TileLayer url='https://t1.openseamap.org/seamark/{z}/{x}/{y}.png' />

        {this.props.buoys.filter((buoy) => buoy.any_lat() != undefined).map(this.BuoyMarker)}

        {this.state.myself != undefined &&
          <Marker key="myself" position={[this.state.myself[0], this.state.myself[1]]} icon={myselfIcon}>
            <Tooltip>
              Your position.
            </Tooltip>
          </Marker>
        }

        <this.MapController />

      </MapContainer>
    );
  }
}

