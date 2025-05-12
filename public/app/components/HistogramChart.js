import { html, useEffect, useRef } from "https://esm.sh/htm/preact/standalone";

function HistogramChart({ metricSample }) {
  const chartRef = useRef(null);

  useEffect(() => {
    const latestSample = metricSample.metrics[metricSample.metrics.length - 1];
    const unit = metricSample.unit || "count";
    if (!latestSample || !latestSample.buckets) return;

    const buckets = latestSample.buckets;
    const bucketKeys = Object.keys(buckets)
      .filter((key) => key !== "+Inf")
      .map(Number)
      .sort((a, b) => a - b);
    const bucketCounts = bucketKeys.map((key) =>
      Number.parseInt(buckets[key], 10),
    );

    const data = bucketKeys.map((key, index) => {
      const lowerBound = index === 0 ? 0 : bucketKeys[index - 1];
      return {
        x: `${lowerBound}–${key}`,
        y: bucketCounts[index] - (bucketCounts[index - 1] || 0),
      };
    });

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
        type: "bar",
        height: 350,
        toolbar: {
          show: true,
        },
        animations: {
          enabled: false,
        },
      },
      plotOptions: {
        bar: {
          columnWidth: "100%",
        },
      },
      dataLabels: {
        enabled: false,
      },
      series: [
        {
          name: "Frequency",
          data: data,
        },
      ],
      xaxis: {
        title: {
          text: "Value Range",
        },
      },
      yaxis: {
        title: {
          text: unit,
        },
      },
      tooltip: {
        x: {
          formatter: (val) => {
            return `Range: ${val}`;
          },
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

export default HistogramChart;
