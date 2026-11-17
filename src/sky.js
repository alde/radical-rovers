import * as THREE from 'three';

const SKY_RADIUS = 120000;
const STAR_COUNT = 3000;
const SUN_DISTANCE = 100000, SUN_SIZE = 1800;

/** Lights, gradient sky dome, sun disc and (on airless worlds) stars. Follows the camera. */
export class Sky {
  constructor(scene, planet) {
    this.group = new THREE.Group();
    scene.add(this.group);
    scene.fog = new THREE.Fog(planet.fog.color, planet.fog.near, planet.fog.far);
    scene.background = new THREE.Color(planet.sky.horizon);
    const sunDir = new THREE.Vector3(...planet.sun.direction).normalize();
    this.addLights(scene, planet, sunDir);
    this.group.add(buildDome(planet.sky), buildSun(sunDir));
    if (planet.stars) this.group.add(buildStars());
  }

  addLights(scene, { sun, ambient }, sunDir) {
    this.sunLight = new THREE.DirectionalLight(sun.color, sun.intensity);
    this.sunLight.position.copy(sunDir);
    scene.add(this.sunLight, new THREE.HemisphereLight(ambient.sky, ambient.ground, ambient.intensity));
  }

  update(camera) {
    this.group.position.copy(camera.position);
  }
}

function buildDome({ zenith, horizon }) {
  const geometry = new THREE.SphereGeometry(SKY_RADIUS, 32, 16);
  const top = new THREE.Color(zenith), bottom = new THREE.Color(horizon), c = new THREE.Color();
  const pos = geometry.attributes.position, colors = new Float32Array(pos.count * 3);
  for (let i = 0; i < pos.count; i++) {
    c.lerpColors(bottom, top, Math.max(0, pos.getY(i) / SKY_RADIUS) ** 0.6).toArray(colors, i * 3);
  }
  geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  const material = new THREE.MeshBasicMaterial({ vertexColors: true, side: THREE.BackSide, fog: false, depthWrite: false });
  const dome = new THREE.Mesh(geometry, material);
  dome.renderOrder = -2;
  return dome;
}

function buildSun(direction) {
  const sun = new THREE.Mesh(new THREE.SphereGeometry(SUN_SIZE, 16, 8), new THREE.MeshBasicMaterial({ color: 0xfffbe8, fog: false }));
  sun.position.copy(direction).multiplyScalar(SUN_DISTANCE);
  return sun;
}

function buildStars() {
  const positions = new Float32Array(STAR_COUNT * 3), v = new THREE.Vector3();
  for (let i = 0; i < STAR_COUNT; i++) v.randomDirection().multiplyScalar(SKY_RADIUS * 0.9).toArray(positions, i * 3);
  const geometry = new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(positions, 3));
  const stars = new THREE.Points(geometry, new THREE.PointsMaterial({ color: 0xffffff, size: 1.6, sizeAttenuation: false, fog: false }));
  stars.renderOrder = -1;
  return stars;
}
