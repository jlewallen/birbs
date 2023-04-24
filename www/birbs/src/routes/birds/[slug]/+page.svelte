<script>
  import { onMount } from "svelte";
  import moment from "moment";
  import _ from "lodash";

  /** @type {import('./$types').PageData} */
  export let data;

  const dateToString = (/** @type Date */ date) => moment(date).toISOString();

  const byDate = _.groupBy(data.daily, (d) => dateToString(d.date));
  const byHour = _.groupBy(data.hourly, (h) => h.number);
  const start = _.min(
    data.daily.map((/** @type {{ date: String; }} */ r) => new Date(r.date))
  );
  const end = _.max(
    data.daily.map((/** @type {{ date: String; }} */ r) => new Date(r.date))
  );

  const days = (end - start) / 86400 / 1000;
  const dates = _.range(0, days + 1).map((days) => {
    const date = new Date(start);
    date.setDate(date.getDate() + days);
    return dateToString(date);
  });

  // console.log(_.keys(byDate));
  // console.log(dates);
  // console.log(start, end, days, dates);

  function dailyChart() {
    const detections = dates.map(function (i) {
      if (byDate[i]) {
        return byDate[i][0].detections;
      }
      return 0;
    });

    const data = {
      x: dates,
      y: detections,
      marker: {
        color: "#bc4b51",
      },
    };

    const layout = {
      height: 225,
      showlegend: false,
      usecontainerwidth: true,
      paper_bgcolor: "rgba(0, 0, 0, 0)",
      plot_bgcolor: "rgba(0, 0, 0, 0)",
      xaxis: { gridcolor: "darkgreen" },
      yaxis: { gridcolor: "darkgreen" },
      margin: {
        b: 40,
        r: 20,
        l: 40,
        t: 40,
      },
    };

    const config = {};

    // @ts-ignore
    Plotly.newPlot("daily", [data], layout, config);
  }

  function hourlyChart() {
    const totalDetections = _.sum(
      _.map(_.flatten(_.values(byHour)), (h) => h.detections)
    );

    const detections = _.range(0, 24).map(function (i) {
      if (byHour[i]) {
        return byHour[i][0].detections / totalDetections;
      }
      return 0;
    });

    const theta = _.range(0, 24).map(function (i) {
      return i * (360 / 24);
    });

    const data = {
      r: detections,
      theta: theta,
      type: "barpolar",
      marker: {
        color: "darkgreen",
      },
    };

    const ticks = {
      values: [
        0, 15, 35, 45, 60, 75, 90, 105, 120, 135, 150, 165, 180, 195, 210, 225,
        240, 255, 270, 285, 300, 315, 330, 345,
      ],
      text: [
        "12am",
        "1am",
        "2am",
        "3am",
        "4am",
        "5am",
        "6am",
        "7am",
        "8am",
        "9am",
        "10am",
        "11am",
        "12pm",
        "1pm",
        "2pm",
        "3pm",
        "4pm",
        "5pm",
        "6pm",
        "7pm",
        "8pm",
        "9pm",
        "10pm",
        "11pm",
      ],
    };

    const layout = {
      autosize: false,
      width: 500,
      height: 500,
      showlegend: false,
      paper_bgcolor: "rgba(0, 0, 0, 0)",
      plot_bgcolor: "rgba(0, 0, 0, 0)",
      polar: {
        radialaxis: {
          tickfont_size: 15,
          showticklabels: false,
          hoverformat: "%{percent}",
        },
        angularaxis: {
          tickfont_size: 15,
          rotation: -90,
          direction: "clockwise",
          tickmode: "array",
          tickvals: ticks.values,
          ticktext: ticks.text,
        },
      },
      margin: {},
    };

    const config = {};

    // @ts-ignore
    Plotly.newPlot("hourly", [data], layout, config);
  }

  onMount(() => {
    // Is it just me or is this super freaking awkward. I'm at a loss for a
    // better way, though and this at least works.
    const script = document.createElement("script");
    script.src = "https://cdn.plot.ly/plotly-latest.min.js";
    document.head.append(script);

    script.onload = function () {
      dailyChart();
      hourlyChart();
    };
  });
</script>

<svelte:head>
  <title>{data.bird.common_name}</title>
  <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
</svelte:head>

<section>
  <div id="daily" />

  <div class="details">
    <div class="details-row">
      <div class="photo">
        <img src={data.bird.photo_url} alt="A bird" />
      </div>
      <div class="info">
        <h1>{data.bird.common_name}</h1>

        <div id="hourly" />
      </div>
    </div>
    <div class="details-row">
      {data.files.detections.total} Detections
    </div>
  </div>

  <div class="body">
    {#await data.files}
      Loading...
    {:then value}
      {#each value.files as file}
        {#if file.available}
          <div class="detection file">
            <div class="header">
              <div class="when">
                {moment(file.when).format("dddd, MMMM Do YYYY, h:mm:ss a")}
              </div>
              <div class="confidence">
                {file.confidence}
              </div>
            </div>
            <div class="video">
              <!-- svelte-ignore a11y-media-has-caption -->
              <video
                style="margin-top: 10px"
                controls
                poster={file.spectrogram_url}
                preload="none"
                title={file.audio_url}><source src={file.audio_url} /></video
              >
            </div>
          </div>
        {/if}
      {/each}
    {:catch error}
      {error.message}
    {/await}
  </div>
</section>

<style>
  #daily {
    height: 225px;
  }

  #hourly {
    height: 500px;
  }

  .detection .header {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: end;
  }

  .detection .confidence {
    color: darkgreen;
    font-size: 14px;
    margin-right: 2em;
  }

  .detection .when {
    margin-left: 1.5em;
    font-size: 14pt;
  }

  .detection,
  .file {
    margin-top: 1em;
  }

  .detection .video {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .details {
    background-color: white;
    border: 1px solid black;
  }

  .details-row {
    display: flex;
    flex-direction: row;
    padding: 1em;
    margin-bottom: 1em;
    justify-content: space-between;
  }

  .details-row .photo {
    width: 100%;
    align-items: center;
    display: flex;
    justify-content: center;
  }

  .details-row .photo img {
    border: 1px solid #0d0d0d;
    /* object-fit: scale-down; */
    width: 100%;
  }

  .details-row .info {
    min-width: 500px;
  }

  .details-row .info h1 {
    margin: 0em 0em 0em 0em;
    text-align: center;
  }
</style>
