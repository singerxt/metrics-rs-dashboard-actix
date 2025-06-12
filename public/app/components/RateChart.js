import { html, useEffect, useRef } from "https://esm.sh/htm/preact/standalone";
import { groupByLabelType, normalizeFloat } from "../common/metricUtils.js";

/**
 * RateChart component for displaying rate metrics (per-second calculations)
 * @param {Object} props - Component props
 * @param {Object} props.metricSample - The metric sample data containing rate information
 * @returns {JSX.Element} Rendered rate chart
 */
function RateChart({ metricSample }) {
  const chartRef = useRef(null);

  useEffect(() => {
    const dataByLabelType = groupByLabelType(metricSample.metrics);
    const unit = metricSample.unit || "per_second";

    // Determine the base unit for rate display
    let rateUnit = "per second";
    if (unit.includes("bytes")) {
      rateUnit = "bytes/sec";
    } else if (unit.includes("requests")) {
      rateUnit = "requests/sec";
    } else if (unit.includes("count")) {
      rateUnit = "count/sec";
    } else if (unit !== "per_second") {
      rateUnit = `${unit}/sec`;
    }

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
        text: `${metricSample.help} (Rate)`,
        align: "left",
        style: {
          fontSize: "16px",
          color: "#fff",
        },
      },
      chart: {
        type: "area",
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
          text: rateUnit,
        },
        labels: {
          formatter: (value) => {
            // Format rate values with appropriate precision
            if (value === 0) return "0";
            if (value < 0.01) return value.toExponential(2);
            if (value < 1) return value.toFixed(3);
            if (value < 10) return value.toFixed(2);
            return normalizeFloat(value);
          },
        },
        min: 0, // Rate charts should start from 0
      },
      tooltip: {
        x: {
          format: "HH:mm:ss:ms",
        },
        y: {
          formatter: (value) => {
            if (value === 0) return "0 " + rateUnit;
            if (value < 0.01) return value.toExponential(2) + " " + rateUnit;
            if (value < 1) return value.toFixed(3) + " " + rateUnit;
            if (value < 10) return value.toFixed(2) + " " + rateUnit;
            return normalizeFloat(value) + " " + rateUnit;
          },
        },
      },
      fill: {
        type: "gradient",
        gradient: {
          shade: "dark",
          type: "horizontal",
          shadeIntensity: 0.5,
          gradientToColors: ["#17a2b8"], // Teal color for rate charts
          inverseColors: true,
          opacityFrom: 0.7,
          opacityTo: 0.3,
          stops: [0, 100],
        },
      },
      stroke: {
        curve: "smooth",
        width: 2,
      },
      theme: {
        enabled: true,
        color: "#17a2b8", // Teal theme for rate charts
        shadeTo: "dark",
        shadeIntensity: 0.65,
      },
      grid: {
        borderColor: "#40475D",
      },
      markers: {
        size: 0,
        hover: {
          size: 5,
        },
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

export default RateChart;
