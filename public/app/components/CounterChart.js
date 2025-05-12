import { html, useEffect, useRef } from "https://esm.sh/htm/preact/standalone";
import { groupByLabelType } from "../common/metricUtils.js";

function CounterChart({ metricSample }) {
  const chartRef = useRef(null);

  useEffect(() => {
    const dataByLabelType = groupByLabelType(metricSample.metrics);
    const unit = metricSample.unit || "count";
    const options = {
      title: {
        text: metricSample.name,
        align: "left",
        style: {
          fontSize: "24px",
          color: "#fff",
        },
      },
      subtitle: {
        text: `${metricSample.help}`,
        align: "left",
        style: {
          fontSize: "16px",
          color: "#fff",
        },
      },
      chart: {
        type: "line",
        height: 350,
        animations: {
          enabled: false,
        },
        toolbar: {
          tools: {
            download: true,
            selection: false,
            zoom: false,
            zoomin: false,
            zoomout: false,
            pan: false,
            reset: false,
            customIcons: [],
          },
        },
      },
      series: dataByLabelType,
      xaxis: {
        type: "datetime",
        title: {
          text: "Time",
        },
      },
      yaxis: {
        title: {
          text: unit,
        },
      },
      tooltip: {
        x: {
          format: "HH:mm:ss:ms",
        },
      },
      theme: {
        enabled: true,
        color: "#255aee",
        shadeTo: "dark",
        shadeIntensity: 0.65,
      },
    };

    const chart = new ApexCharts(chartRef.current, options);
    chart.render();

    return () => {
      chart.destroy();
    };
  }, [JSON.stringify(metricSample)]);

  return html`<div ref=${chartRef}></div>`;
}

export default CounterChart;
