import _ from "lodash";

// since there's no dynamic data here, we can prerender
// it so that it gets served as a static asset in production
export const prerender = true;

/** @type {import('./$types').PageLoad} */
export async function load({ params }) {
  const res = await fetch(`http://127.0.0.1:3100/by-day-and-common-name.json`);

  if (res.ok) {
    return {
      query: res.json().then((rows) => {
        const query = _(rows)
          .groupBy((row) => row.when)
          .map((birds, day) => {
            return {
              date: new Date(day),
              birds: _(birds)
                .map((b) => {
                  console.log(b);
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
          .value();
        // console.log(query);
        return query;
      }),
    };
  } else {
    throw new Error("Oops");
  }
}
