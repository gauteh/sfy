import {Component} from 'react';
import L from 'leaflet';

import {Buoy, OmbBuoy} from '/models';

import './marker-me.png';
import './BuoyIndex.scss';
import 'leaflet/dist/leaflet.css';
import 'leaflet/dist/images/marker-icon-2x.png';
import 'leaflet/dist/images/marker-shadow.png';
import 'leaflet/dist/images/marker-shadow.png';

const MAPBOX_TOKEN: string = 'pk.eyJ1IjoiZ2F1dGVoIiwiYSI6ImNreWZ2MWd4NjBxNnoyb3M4eWRqNjlmMGMifQ.m-5Q9BBf2yQxp1fGStxYRg';

interface Props {
  buoys: Array<Buoy | OmbBuoy>;
}

interface State {
}

export class BuoyMap
  extends Component<Props, State>
{

  public state = {};
  map: any = {};
  myselfMarker: L.Marker;
  markers: [string, any][];

  constructor(props: Props, context) {
    super(props, context);

    if (navigator.geolocation) {
      navigator.geolocation.watchPosition(this.updateMyPosition);
    }
  }

  componentDidMount() {
    console.log("loading leaflet..");
    this.map = L.map('map').setView([60.11304848114283, 2.3882482939071434], 5);

    L.tileLayer('https://api.mapbox.com/styles/v1/{id}/tiles/{z}/{x}/{y}?access_token={access_token}', {
      maxZoom: 18,
      attribution: 'Map data &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors, ' +
        'Imagery © <a href="https://www.mapbox.com/">Mapbox</a>',
      id: 'mapbox/light-v10',
      tileSize: 512,
      zoomOffset: -1,
      access_token: MAPBOX_TOKEN
    }).addTo(this.map);

    L.tileLayer('https://t1.openseamap.org/seamark/{z}/{x}/{y}.png').addTo(this.map);
  }

  componentDidUpdate = (lastprops) => {
    if (this.markers !== undefined) {
      for (const m of this.markers) {
        this.map.removeLayer(m[1]);
      }
    }

    this.markers = this.props.buoys.map((buoy) => {
        let marker = L.marker([buoy.any_lat(), buoy.any_lon()]).addTo(this.map);
        marker.bindTooltip(`${buoy.sn} (${buoy.dev})`);
        return [buoy.dev, marker];
      });
  }

  public updateMyPosition = (position) => {
    console.log("Got new position:" + position);
    if (this.myselfMarker === undefined) {
      const icon = L.icon({
        iconUrl: 'public/icons/marker-me.png',

        iconSize:     [30, 38], // size of the icon
        // shadowSize:   [50, 64], // size of the shadow
        iconAnchor:   [15, 38], // point of the icon which will correspond to marker's location
        // shadowAnchor: [4, 62],  // the same for the shadow
        // popupAnchor:  [-3, -76] // point from which the popup should open relative to the iconAnchor
      });

      this.myselfMarker = L.marker([position.coords.latitude, position.coords.longitude], {icon: icon}).addTo(this.map);
      this.myselfMarker.bindTooltip('Your position');
    } else {
      this.myselfMarker.setLatLng([position.coords.latitude, position.coords.longitude]);
    }
  }

  public focus = (buoy) => {
    console.log("Focusing: " + buoy);

    this.map.flyTo([buoy.any_lat(), buoy.any_lon()], 11);
  }

  public render() {
    return (
      <div id="map" class="container-fluid">
      </div>
    );
  }
}


