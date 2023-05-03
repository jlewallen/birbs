import { dev, browser } from '$app/environment';
import _ from "lodash";

/** @type {import('./$types').PageLoad} */
export async function load({ fetch, params }) {
  const recently = fetch(`http://127.0.0.1:3100/recently.json`)
    .then((res) => res.json())
    .then((resp) => {
      const detections = resp.detections;
      const summarized = _(detections).groupBy((d) => d.common_name).map((g, common_name) =>{
        const by_time = _(g).sortBy((d) => d.when).reverse().value();
        const by_confidence = _(g).sortBy((d) => d.confidence).reverse().value();

        return {
          common_name: common_name,
          total_24h: g.length,
          last: by_time[0],
          best: by_confidence[0],
        }
      }).value();
      return {
        summarized,
        detections,
      };
    });

  return {
    recently,
  };
}
