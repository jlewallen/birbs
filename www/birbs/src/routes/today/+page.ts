import { dev, browser } from '$app/environment';

/** @type {import('./$types').PageLoad} */
export async function load({ fetch, params }) {
  const recently = fetch(`http://127.0.0.1:3100/recently.json`)
    .then((res) => res.json())
    .then((resp) => {
      console.log("detections", resp.detections)
      return {
        detections: resp.detections
      };
    });

  return {
    recently,
  };
}
