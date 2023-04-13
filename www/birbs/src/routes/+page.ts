import { dev, browser } from '$app/environment';
import _ from "lodash";

// $: if (browser) document.body.classList.toggle('green', true);

// since there's no dynamic data here, we can prerender
// it so that it gets served as a static asset in production
export const prerender = true;

function fixed_width_date(date: Date): String {
  return [
    date.getUTCFullYear(),
    ("0" + (date.getUTCMonth() + 1)).slice(-2),
    ("0" + date.getDate()).slice(-2),
  ].join("-");
}

/** @type {import('./$types').PageLoad} */
export async function load({ fetch, params }) {
  const res = await fetch(`http://127.0.0.1:3100/by-day-and-common-name.json`);

  if (res.ok) {
    return {
      query: res.json().then((rows) => {
        const query = _(rows)
          .groupBy((row) => row.when)
          .map((birds, day) => {
            const date = new Date(day);
            const dateString = fixed_width_date(date);
            const url = `http://192.168.0.164/Charts/Combo-${dateString}.png?nocache=${Date.now()}`;

            return {
              date: date,
              histogram_url: url,
              birds: _(birds)
                .map((b) => {
                  return {
                    when: b.when,
                    common_name: b.common_name,
                    total: b.total,
                    average_confidence: {
                      display: b.average_confidence.toLocaleString("en", {
                        notation: "compact",
                        compactDisplay: "short",
                      }),
                    },
                  };
                })
                .orderBy((b) => b.total)
                .reverse()
                .value(),
            };
          })
          .orderBy((row) => row.date)
          .reverse()
          .take(31)
          .value();
        // console.log(query);
        return query;
      }),
    };
  } else {
    throw new Error("Oops");
  }
}
