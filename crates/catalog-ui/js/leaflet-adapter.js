/**
 * Ride Atlas Leaflet Map Adapter
 * Implements the map contract between WebAssembly/Rust and Leaflet.
 *
 * Contract:
 * - createMap(elementId, config) -> Adapter
 * - setOverview(key, feature)
 * - removeOverview(key)
 * - setPlaces(points)
 * - setSelection(key, feature)
 * - clearSelection()
 * - fitBounds(bounds)
 * - resize()
 * - destroy()
 */

(function (root, factory) {
  if (typeof define === 'function' && define.amd) {
    define([], factory);
  } else if (typeof module === 'object' && module.exports) {
    module.exports = factory();
  } else {
    root.RideAtlasMap = factory();
    root.createMap = root.RideAtlasMap.createMap;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
  'use strict';

  if (typeof document !== 'undefined') {
    const connection = document.createElement('link');
    connection.rel = 'preconnect';
    connection.href = 'https://server.arcgisonline.com';
    connection.crossOrigin = 'anonymous';
    document.head.appendChild(connection);
  }

  const DEFAULT_OVERVIEW_BOUNDS = [
    [-100.2055, 29.6695],
    [-96.5996, 30.9865]
  ];

  // EPA / TPWD Level III Ecoregion 30 (Edwards Plateau & Balcones Escarpment) boundary
  const HILL_COUNTRY_BOUNDARY_GEOJSON = {"type":"Feature","properties":{"name":"Texas Hill Country (Edwards Plateau)","source":"EPA / TPWD Level III Ecoregion 30 (Edwards Plateau & Balcones Escarpment)"},"geometry":{"type":"Polygon","coordinates":[[[-101.1375,32.0225],[-101.1483,32.0256],[-101.1579,32.0312],[-101.1857,32.0647],[-101.2003,32.0752],[-101.2377,32.0863],[-101.2809,32.0919],[-101.2994,32.0962],[-101.3119,32.1026],[-101.3338,32.1205],[-101.3515,32.1304],[-101.3847,32.138],[-101.4111,32.1499],[-101.4147,32.1507],[-101.4169,32.1401],[-101.422,32.1337],[-101.4423,32.1241],[-101.453,32.1161],[-101.4555,32.1085],[-101.4528,32.1034],[-101.4232,32.0801],[-101.4124,32.0671],[-101.4105,32.0619],[-101.4128,32.0529],[-101.426,32.0357],[-101.4369,31.9985],[-101.4365,31.9699],[-101.4345,31.9604],[-101.4352,31.9415],[-101.44,31.9338],[-101.4446,31.931],[-101.4534,31.9314],[-101.4909,31.9405],[-101.4996,31.9408],[-101.508,31.9364],[-101.5108,31.9327],[-101.5115,31.9257],[-101.5083,31.9151],[-101.5058,31.9116],[-101.4875,31.8986],[-101.4863,31.8901],[-101.494,31.8796],[-101.5019,31.8737],[-101.5333,31.8648],[-101.5452,31.8585],[-101.5476,31.854],[-101.5482,31.8465],[-101.5417,31.8313],[-101.5357,31.825],[-101.5199,31.8212],[-101.5144,31.817],[-101.5061,31.7779],[-101.5033,31.7729],[-101.4937,31.7661],[-101.476,31.7638],[-101.4593,31.7587],[-101.4486,31.7585],[-101.4186,31.7667],[-101.3895,31.7722],[-101.371,31.7808],[-101.3551,31.782],[-101.3506,31.7814],[-101.3288,31.7701],[-101.3115,31.7635],[-101.2972,31.755],[-101.2764,31.7328],[-101.2626,31.71],[-101.2581,31.6963],[-101.2575,31.6862],[-101.2683,31.611],[-101.2761,31.6056],[-101.2874,31.6035],[-101.3087,31.6051],[-101.3383,31.6146],[-101.3497,31.612],[-101.3561,31.603],[-101.3685,31.5715],[-101.3773,31.5589],[-101.3832,31.5535],[-101.4193,31.5333],[-101.4494,31.5089],[-101.46,31.4914],[-101.4652,31.4747],[-101.4716,31.4668],[-101.4794,31.4616],[-101.5078,31.4546],[-101.5351,31.452],[-101.5408,31.4491],[-101.562,31.4303],[-101.5915,31.4216],[-101.6306,31.4162],[-101.6524,31.4211],[-101.6616,31.4214],[-101.6954,31.416],[-101.7011,31.4131],[-101.7156,31.3986],[-101.7248,31.3725],[-101.7337,31.3602],[-101.7582,31.3482],[-101.7768,31.3303],[-101.801,31.3135],[-101.8156,31.3075],[-101.8602,31.3023],[-101.8882,31.2961],[-101.92,31.2765],[-101.9686,31.2502],[-101.9787,31.2469],[-101.9923,31.247],[-101.9917,31.2367],[-101.9817,31.2141],[-101.9805,31.2063],[-101.9846,31.1962],[-102.0001,31.183],[-102.0046,31.1764],[-102.0055,31.1674],[-101.9992,31.155],[-101.997,31.1375],[-101.9979,31.1298],[-102.0039,31.1141],[-102.004,31.1074],[-101.9997,31.0831],[-101.9932,31.0625],[-101.9905,31.0361],[-101.9833,31.0207],[-101.9874,31.0061],[-101.9744,30.9979],[-101.9678,30.9886],[-101.9613,30.989],[-101.9419,30.9973],[-101.9288,30.9892],[-101.9088,30.9853],[-101.8975,30.9654],[-101.8945,30.9629],[-101.8866,30.9618],[-101.8747,30.9661],[-101.8659,30.9626],[-101.8612,30.9536],[-101.8616,30.9315],[-101.8581,30.9239],[-101.848,30.9125],[-101.8349,30.9078],[-101.8391,30.898],[-101.8451,30.8911],[-101.8544,30.8846],[-101.8563,30.8788],[-101.8518,30.8684],[-101.8322,30.8457],[-101.824,30.833],[-101.8205,30.8228],[-101.8238,30.798],[-101.8152,30.7878],[-101.8144,30.784],[-101.8207,30.7602],[-101.8099,30.7352],[-101.7873,30.7044],[-101.7835,30.6908],[-101.779,30.6829],[-101.7685,30.6692],[-101.7562,30.6582],[-101.7396,30.6563],[-101.7251,30.6627],[-101.7091,30.6641],[-101.6562,30.6484],[-101.6466,30.6373],[-101.6437,30.6243],[-101.6433,30.5893],[-101.6378,30.5606],[-101.6388,30.5544],[-101.6477,30.5418],[-101.6737,30.5174],[-101.6788,30.4988],[-101.6927,30.4656],[-101.7009,30.4599],[-101.7168,30.4619],[-101.7204,30.4567],[-101.7234,30.4368],[-101.7327,30.4291],[-101.7338,30.4254],[-101.7311,30.419],[-101.7268,30.4152],[-101.7014,30.4021],[-101.6898,30.3922],[-101.6828,30.3775],[-101.6673,30.3644],[-101.6589,30.3513],[-101.6585,30.3455],[-101.6619,30.3405],[-101.669,30.3366],[-101.6833,30.3392],[-101.6963,30.3388],[-101.7082,30.3364],[-101.7192,30.3308],[-101.7316,30.3197],[-101.7433,30.3031],[-101.7461,30.2877],[-101.7438,30.2835],[-101.7364,30.2779],[-101.7174,30.272],[-101.6837,30.2544],[-101.6655,30.2389],[-101.653,30.2196],[-101.6414,30.2132],[-101.6256,30.2112],[-101.6075,30.2045],[-101.5898,30.1844],[-101.5809,30.1775],[-101.572,30.174],[-101.5538,30.1848],[-101.5494,30.1857],[-101.5393,30.1803],[-101.5334,30.1708],[-101.5218,30.1707],[-101.5182,30.1689],[-101.511,30.1616],[-101.498,30.1527],[-101.495,30.146],[-101.4933,30.1322],[-101.4874,30.1278],[-101.4724,30.127],[-101.4508,30.1196],[-101.4373,30.1206],[-101.4069,30.1326],[-101.3809,30.1537],[-101.3648,30.1558],[-101.3367,30.1634],[-101.3275,30.161],[-101.3242,30.1509],[-101.3185,30.1429],[-101.3112,30.1373],[-101.2909,30.1291],[-101.2739,30.1064],[-101.2711,30.0937],[-101.2772,30.0727],[-101.2755,30.0613],[-101.262,30.0398],[-101.2504,30.0313],[-101.2449,30.0246],[-101.2307,29.9861],[-101.217,29.9652],[-101.153,29.9078],[-101.1235,29.867],[-101.1056,29.8549],[-101.0815,29.8444],[-101.0736,29.8349],[-101.0769,29.8177],[-101.072,29.7898],[-101.0747,29.7804],[-101.0875,29.7665],[-101.0845,29.7458],[-101.0757,29.7323],[-101.0531,29.7083],[-101.0427,29.6917],[-101.028,29.6782],[-101.0096,29.6527],[-100.996,29.6453],[-100.9886,29.644],[-100.9677,29.6453],[-100.942,29.6518],[-100.9338,29.6557],[-100.9196,29.6679],[-100.8928,29.6759],[-100.8688,29.6975],[-100.8648,29.7087],[-100.8651,29.7213],[-100.8725,29.7372],[-100.8822,29.7413],[-100.8854,29.7415],[-100.8948,29.7372],[-100.9303,29.7311],[-100.9558,29.7307],[-100.9581,29.736],[-100.9557,29.7394],[-100.9454,29.7412],[-100.9405,29.7468],[-100.9487,29.7681],[-100.9455,29.7766],[-100.9406,29.7812],[-100.8963,29.7949],[-100.8822,29.7958],[-100.8545,29.8038],[-100.8275,29.8062],[-100.7992,29.8039],[-100.777,29.799],[-100.7375,29.7967],[-100.7071,29.8057],[-100.685,29.8251],[-100.6701,29.8296],[-100.664,29.8257],[-100.6574,29.8161],[-100.6522,29.8123],[-100.624,29.8069],[-100.6143,29.8013],[-100.6069,29.794],[-100.6011,29.7825],[-100.6001,29.7761],[-100.6019,29.7656],[-100.6074,29.7556],[-100.6166,29.7489],[-100.6325,29.7439],[-100.6542,29.7294],[-100.6619,29.7299],[-100.6819,29.7368],[-100.7011,29.7304],[-100.7136,29.7323],[-100.7404,29.7404],[-100.7533,29.7398],[-100.7695,29.7307],[-100.7774,29.7175],[-100.7775,29.6979],[-100.7729,29.6852],[-100.7322,29.6456],[-100.7148,29.6338],[-100.7014,29.6282],[-100.6946,29.6211],[-100.6831,29.6156],[-100.67,29.6151],[-100.6571,29.6234],[-100.6482,29.6333],[-100.6411,29.6362],[-100.6358,29.6326],[-100.6349,29.6248],[-100.6377,29.6115],[-100.6314,29.6011],[-100.6021,29.5904],[-100.5944,29.5827],[-100.5847,29.5765],[-100.5653,29.5683],[-100.5597,29.5691],[-100.5488,29.5759],[-100.544,29.5769],[-100.5403,29.5751],[-100.5331,29.5647],[-100.5217,29.5606],[-100.5061,29.5672],[-100.4905,29.5707],[-100.4781,29.5701],[-100.4622,29.5664],[-100.4401,29.568],[-100.4324,29.5712],[-100.4135,29.5844],[-100.4068,29.5861],[-100.4002,29.5853],[-100.3873,29.5773],[-100.3642,29.5694],[-100.3573,29.5619],[-100.3565,29.5588],[-100.3588,29.5523],[-100.3723,29.5456],[-100.3767,29.5408],[-100.3751,29.5318],[-100.3636,29.5284],[-100.3353,29.5277],[-100.3263,29.5258],[-100.3214,29.5235],[-100.3109,29.5129],[-100.3014,29.5082],[-100.286,29.5029],[-100.2254,29.4892],[-100.22,29.489],[-100.2089,29.4956],[-100.2034,29.4961],[-100.1988,29.4902],[-100.197,29.4715],[-100.1806,29.4651],[-100.172,29.4552],[-100.1604,29.45],[-100.1036,29.4541],[-100.0881,29.4484],[-100.0713,29.4372],[-100.0544,29.4329],[-99.9915,29.4532],[-99.987,29.4488],[-99.9855,29.4426],[-99.9927,29.4111],[-99.9901,29.3975],[-99.9759,29.3832],[-99.9677,29.3781],[-99.9626,29.3698],[-99.9553,29.3655],[-99.945,29.3414],[-99.9391,29.3335],[-99.9291,29.3279],[-99.908,29.3217],[-99.8594,29.3226],[-99.8192,29.3389],[-99.7584,29.3718],[-99.7325,29.3835],[-99.7096,29.3983],[-99.6917,29.405],[-99.673,29.4152],[-99.6568,29.4217],[-99.6386,29.4246],[-99.6265,29.4291],[-99.5825,29.4589],[-99.564,29.465],[-99.5431,29.4688],[-99.5018,29.4659],[-99.4781,29.4746],[-99.457,29.4856],[-99.4102,29.4987],[-99.3714,29.4985],[-99.3544,29.5053],[-99.3375,29.5052],[-99.3053,29.5131],[-99.2839,29.5144],[-99.2487,29.5081],[-99.2435,29.5048],[-99.2272,29.4865],[-99.2201,29.4838],[-99.2056,29.4834],[-99.1609,29.5013],[-99.1471,29.5021],[-99.1246,29.4983],[-99.1084,29.5036],[-99.0911,29.5049],[-99.0749,29.5008],[-99.0567,29.4929],[-99.0472,29.4873],[-99.0287,29.4693],[-99.0209,29.4664],[-99.014,29.4666],[-98.9607,29.4762],[-98.8741,29.5178],[-98.7853,29.5444],[-98.6472,29.5795],[-98.6054,29.5837],[-98.5894,29.5825],[-98.5833,29.5803],[-98.5632,29.5619],[-98.549,29.5577],[-98.5062,29.5561],[-98.4792,29.5613],[-98.4487,29.5723],[-98.4132,29.5912],[-98.3872,29.6006],[-98.3534,29.6066],[-98.3274,29.6161],[-98.2919,29.6256],[-98.2594,29.6381],[-98.162,29.7022],[-98.1391,29.7279],[-98.1098,29.7487],[-98.0577,29.7953],[-98.0438,29.8058],[-98.0109,29.8257],[-97.9882,29.8432],[-97.9778,29.8524],[-97.9621,29.8711],[-97.9257,29.8968],[-97.9098,29.9138],[-97.905,29.9288],[-97.8986,29.9387],[-97.8958,29.9782],[-97.9018,30.0068],[-97.8942,30.0272],[-97.8859,30.0438],[-97.8722,30.0582],[-97.869,30.0649],[-97.8417,30.0938],[-97.8369,30.1028],[-97.8371,30.1405],[-97.834,30.1549],[-97.8352,30.1764],[-97.8313,30.1963],[-97.8163,30.2174],[-97.8072,30.2259],[-97.7909,30.2369],[-97.7843,30.2463],[-97.7749,30.2758],[-97.7611,30.3091],[-97.7598,30.3359],[-97.7646,30.3573],[-97.7617,30.3672],[-97.7513,30.3794],[-97.7313,30.3947],[-97.7135,30.4146],[-97.7043,30.4304],[-97.697,30.4512],[-97.6904,30.4847],[-97.6889,30.5019],[-97.6984,30.5537],[-97.6981,30.5687],[-97.6881,30.634],[-97.6745,30.668],[-97.6883,30.6718],[-97.7088,30.6827],[-97.7477,30.716],[-97.7601,30.7238],[-97.8173,30.7462],[-97.8622,30.7748],[-97.8828,30.7984],[-97.9096,30.8428],[-97.9321,30.8582],[-97.9462,30.8605],[-97.9922,30.8609],[-98.0083,30.8668],[-98.0242,30.8701],[-98.0613,30.8849],[-98.0914,30.8868],[-98.1186,30.8921],[-98.1403,30.903],[-98.1605,30.9189],[-98.1775,30.9234],[-98.1898,30.9242],[-98.1992,30.9222],[-98.2381,30.9012],[-98.2653,30.8994],[-98.2613,30.9161],[-98.2602,30.9304],[-98.2627,30.9387],[-98.2699,30.9512],[-98.2844,30.9611],[-98.3215,30.9693],[-98.3331,30.978],[-98.3381,30.9872],[-98.3434,31.0123],[-98.3401,31.026],[-98.3376,31.0304],[-98.3258,31.038],[-98.3242,31.0441],[-98.3266,31.0556],[-98.3384,31.0789],[-98.3423,31.098],[-98.3731,31.139],[-98.3966,31.1608],[-98.4122,31.1826],[-98.4271,31.1892],[-98.4494,31.2035],[-98.4611,31.2149],[-98.4737,31.2145],[-98.5223,31.2238],[-98.611,31.2151],[-98.6409,31.2059],[-98.6813,31.1824],[-98.6971,31.1764],[-98.7301,31.1731],[-98.7672,31.1741],[-98.7878,31.1718],[-98.7996,31.1733],[-98.8309,31.182],[-98.8541,31.1913],[-98.8719,31.1943],[-98.9015,31.1945],[-98.9233,31.1969],[-98.9549,31.21],[-98.9607,31.2144],[-98.9685,31.2253],[-98.9694,31.2412],[-98.9674,31.2538],[-98.9705,31.2657],[-98.9768,31.271],[-98.985,31.2739],[-99.0066,31.2767],[-99.0316,31.2714],[-99.0517,31.2694],[-99.0789,31.2583],[-99.1033,31.2574],[-99.1145,31.2545],[-99.2205,31.1851],[-99.254,31.1562],[-99.2757,31.1408],[-99.2839,31.1361],[-99.3004,31.1304],[-99.3169,31.127],[-99.3418,31.1293],[-99.374,31.1275],[-99.3972,31.1319],[-99.4375,31.1511],[-99.4522,31.1533],[-99.4882,31.1488],[-99.498,31.1457],[-99.5437,31.1166],[-99.5523,31.1139],[-99.5699,31.1174],[-99.6073,31.1342],[-99.6122,31.1384],[-99.6138,31.1435],[-99.6015,31.163],[-99.6017,31.1688],[-99.6064,31.1744],[-99.6141,31.1802],[-99.6244,31.1846],[-99.6593,31.1959],[-99.7044,31.2007],[-99.7516,31.1903],[-99.8098,31.1746],[-99.8241,31.1736],[-99.8331,31.1755],[-99.8431,31.1845],[-99.8456,31.1906],[-99.8308,31.2081],[-99.8126,31.2214],[-99.7394,31.2435],[-99.7162,31.2526],[-99.7098,31.2634],[-99.7111,31.2714],[-99.7145,31.2765],[-99.7312,31.2834],[-99.7595,31.2881],[-99.8097,31.3013],[-99.8464,31.3024],[-99.8611,31.3103],[-99.8681,31.3108],[-99.8789,31.3159],[-99.8849,31.3138],[-99.893,31.3017],[-99.9092,31.2959],[-99.9247,31.2859],[-99.9344,31.2852],[-99.9496,31.2965],[-99.9641,31.2929],[-99.9875,31.2763],[-99.9927,31.2659],[-100.0033,31.2554],[-100.0181,31.2446],[-100.0268,31.2408],[-100.0411,31.2418],[-100.0493,31.2467],[-100.0551,31.2554],[-100.0623,31.2785],[-100.0667,31.283],[-100.0734,31.2856],[-100.0812,31.2816],[-100.0901,31.2705],[-100.0896,31.255],[-100.0938,31.2359],[-100.0935,31.2253],[-100.0963,31.2223],[-100.106,31.2274],[-100.1176,31.2418],[-100.1319,31.2527],[-100.1392,31.2692],[-100.1444,31.2747],[-100.1556,31.277],[-100.1753,31.2725],[-100.1856,31.2733],[-100.1953,31.2775],[-100.2144,31.2943],[-100.2257,31.3005],[-100.2425,31.3018],[-100.3071,31.2949],[-100.3339,31.2842],[-100.3519,31.2732],[-100.3605,31.2705],[-100.3738,31.274],[-100.3821,31.2784],[-100.3911,31.2856],[-100.4066,31.3047],[-100.4135,31.31],[-100.4241,31.3133],[-100.4355,31.3109],[-100.4395,31.3075],[-100.4467,31.2801],[-100.4538,31.2687],[-100.4688,31.2597],[-100.4904,31.2423],[-100.4981,31.2391],[-100.5062,31.2395],[-100.513,31.2435],[-100.5211,31.2523],[-100.5424,31.3014],[-100.5609,31.3135],[-100.5845,31.3194],[-100.6082,31.3139],[-100.6324,31.3057],[-100.6369,31.3177],[-100.6333,31.3286],[-100.6217,31.3506],[-100.6206,31.3636],[-100.6376,31.3952],[-100.6421,31.4087],[-100.6401,31.4157],[-100.6318,31.4184],[-100.5781,31.383],[-100.5702,31.3809],[-100.5612,31.3824],[-100.55,31.3872],[-100.5371,31.397],[-100.5295,31.4075],[-100.5247,31.4229],[-100.5244,31.4312],[-100.5274,31.4414],[-100.5456,31.4678],[-100.5526,31.487],[-100.5653,31.5083],[-100.5902,31.5336],[-100.6485,31.5663],[-100.6593,31.5759],[-100.6909,31.609],[-100.7017,31.6242],[-100.7047,31.6369],[-100.7027,31.6439],[-100.6986,31.6487],[-100.6913,31.6524],[-100.6748,31.6538],[-100.6665,31.6498],[-100.6515,31.6357],[-100.6348,31.6284],[-100.5682,31.6175],[-100.54,31.611],[-100.5172,31.6088],[-100.5101,31.6055],[-100.4901,31.5724],[-100.4798,31.5647],[-100.4707,31.563],[-100.4557,31.564],[-100.4214,31.5714],[-100.3884,31.5857],[-100.3769,31.594],[-100.348,31.6187],[-100.3375,31.6343],[-100.3148,31.6519],[-100.3062,31.6619],[-100.3023,31.6683],[-100.2998,31.685],[-100.3046,31.7227],[-100.2937,31.7419],[-100.2958,31.7521],[-100.3055,31.7586],[-100.3228,31.7599],[-100.343,31.7468],[-100.3591,31.7445],[-100.3673,31.7465],[-100.3733,31.7585],[-100.3812,31.7623],[-100.3883,31.7598],[-100.4017,31.7509],[-100.4101,31.7483],[-100.435,31.748],[-100.4537,31.7456],[-100.4636,31.7469],[-100.4769,31.7541],[-100.4885,31.7703],[-100.4999,31.777],[-100.5057,31.7934],[-100.5174,31.8039],[-100.522,31.8118],[-100.5269,31.8161],[-100.5497,31.823],[-100.5608,31.8291],[-100.584,31.8197],[-100.5928,31.8197],[-100.609,31.8264],[-100.6222,31.8402],[-100.6458,31.8443],[-100.6674,31.8649],[-100.6878,31.8714],[-100.6977,31.8809],[-100.7127,31.8793],[-100.7172,31.8804],[-100.723,31.8861],[-100.7308,31.9036],[-100.717,31.9164],[-100.7141,31.9379],[-100.699,31.9438],[-100.6937,31.9503],[-100.6989,31.9658],[-100.7057,31.9724],[-100.7138,31.9698],[-100.7203,31.9618],[-100.7411,31.9553],[-100.7487,31.9583],[-100.7518,31.9626],[-100.7527,31.9684],[-100.7598,31.9737],[-100.764,31.9815],[-100.7687,31.9864],[-100.7756,31.9887],[-100.7963,31.9852],[-100.8139,31.9892],[-100.8374,31.976],[-100.8476,31.9751],[-100.8534,31.9863],[-100.8685,31.997],[-100.8863,32.0053],[-100.8911,32.0104],[-100.9028,32.0166],[-100.9335,32.0272],[-100.9431,32.0338],[-100.9505,32.0362],[-100.9864,32.0276],[-101.0227,32.0124],[-101.0457,32.0154],[-101.0542,32.0198],[-101.0631,32.0279],[-101.0865,32.0319],[-101.0997,32.037],[-101.1093,32.034],[-101.1252,32.0236],[-101.1375,32.0225]]]}};

  function keyToString(key) {
    if (!key) return '';
    return key.kind + ':' + key.id;
  }

  function geoJsonBoundsToLeaflet(bounds) {
    // GeoJSON ordering: [[min_lon, min_lat], [max_lon, max_lat]]
    // Leaflet ordering: [[min_lat, min_lon], [max_lat, max_lon]]
    if (!bounds || bounds.length < 2) return null;
    return [
      [bounds[0][1], bounds[0][0]],
      [bounds[1][1], bounds[1][0]]
    ];
  }

  function leafletBoundsToGeoJson(leafletBounds) {
    return [
      [leafletBounds.getWest(), leafletBounds.getSouth()],
      [leafletBounds.getEast(), leafletBounds.getNorth()]
    ];
  }

  function createMap(elementId, config) {
    config = config || {};

    const container = typeof elementId === 'string'
      ? document.getElementById(elementId)
      : elementId;

    if (!container) {
      throw new Error('Container element not found: ' + elementId);
    }

    // Defensive teardown of any previous map on this container
    if (container._rideAtlasAdapter) {
      container._rideAtlasAdapter.destroy();
    } else if (container._leaflet_id && typeof L !== 'undefined') {
      container._leaflet_id = null;
    }

    if (typeof L === 'undefined') {
      throw new Error('Leaflet library (L) is not loaded');
    }

    let destroyed = false;

    // Canvas renderer for performance with many route lines
    // Keep thin route strokes legible while allowing imprecise mouse/touch selection.
    const canvasRenderer = L.canvas({ padding: 0.5, tolerance: 12 });

    // Initial view coordinates: Central Texas default
    const defaultCenter = [30.45, -98.35];
    const defaultZoom = 8;

    const map = L.map(container, {
      center: defaultCenter,
      zoom: defaultZoom,
      zoomControl: true,
      preferCanvas: true
    });

    map.attributionControl.setPosition('topright');

    // Basemap tile layers matching legacy app
    const baseLayers = {
      'Topographic': L.tileLayer(
        'https://server.arcgisonline.com/ArcGIS/rest/services/World_Topo_Map/MapServer/tile/{z}/{y}/{x}',
        {
          attribution: 'Esri, USGS | &copy; OpenStreetMap contributors',
          maxZoom: 19
        }
      ),
      'Streets': L.tileLayer(
        'https://server.arcgisonline.com/ArcGIS/rest/services/World_Street_Map/MapServer/tile/{z}/{y}/{x}',
        {
          attribution: 'Esri | &copy; OpenStreetMap contributors',
          maxZoom: 19
        }
      ),
      'Dark': L.tileLayer(
        'https://server.arcgisonline.com/ArcGIS/rest/services/Canvas/World_Dark_Gray_Base/MapServer/tile/{z}/{y}/{x}',
        {
          attribution: 'Esri, HERE, Garmin | &copy; OpenStreetMap contributors',
          maxZoom: 16
        }
      ),
      'Satellite': L.tileLayer(
        'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
        {
          attribution: 'Esri, Maxar, Earthstar Geographics',
          maxZoom: 18
        }
      ),
      'OpenStreetMap': L.tileLayer(
        'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
        {
          attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
          maxZoom: 19
        }
      )
    };

    const statusControl = L.control({ position: 'topleft' });
    const tileStatus = document.createElement('div');
    tileStatus.className = 'map-tile-status';
    tileStatus.setAttribute('role', 'status');
    tileStatus.textContent = 'Loading map…';
    statusControl.onAdd = () => tileStatus;
    statusControl.addTo(map);
    Object.values(baseLayers).forEach(layer => {
      layer.on('loading', () => { tileStatus.hidden = false; tileStatus.textContent = 'Loading map…'; });
      layer.on('tileload', () => { tileStatus.hidden = true; });
      layer.on('tileerror', () => { tileStatus.hidden = false; tileStatus.textContent = 'Some map tiles could not load. Try another map layer.'; });
    });
    const keyControl = L.control({ position: 'topleft' });
    keyControl.onAdd = () => {
      const key = document.createElement('div');
      key.className = 'map-key';
      key.textContent = 'Colors identify rides · Select a ride to highlight it';
      L.DomEvent.disableClickPropagation(key);
      return key;
    };
    keyControl.addTo(map);
    baseLayers['Topographic'].addTo(map);

    const hillCountryPane = map.createPane('hillCountryPane');
    hillCountryPane.style.zIndex = '350';
    hillCountryPane.style.pointerEvents = 'none';
    const hillCountryRenderer = L.svg({ pane: 'hillCountryPane', padding: 0.5 });

    const hillCountryHalo = L.geoJSON(HILL_COUNTRY_BOUNDARY_GEOJSON, {
      pane: 'hillCountryPane',
      renderer: hillCountryRenderer,
      interactive: false,
      style: {
        color: 'rgba(255, 255, 255, 0.72)',
        weight: 5,
        opacity: 0.85,
        fill: false,
        interactive: false
      }
    });

    const hillCountryBoundary = L.geoJSON(HILL_COUNTRY_BOUNDARY_GEOJSON, {
      pane: 'hillCountryPane',
      renderer: hillCountryRenderer,
      interactive: false,
      style: {
        color: '#7c4a1e',
        weight: 2.25,
        opacity: 0.88,
        dashArray: '8, 6',
        fillColor: '#b45309',
        fillOpacity: 0.045,
        interactive: false
      }
    });

    const hillCountryLayer = L.layerGroup([
      hillCountryHalo,
      hillCountryBoundary
    ]).addTo(map);

    const overlayLayers = {
      'Hill Country Boundary': hillCountryLayer
    };
    L.control.layers(baseLayers, overlayLayers, { position: 'topright' }).addTo(map);

    const initialOverviewBounds = config.initialBounds || DEFAULT_OVERVIEW_BOUNDS;
    let lastOverviewBounds = initialOverviewBounds;
    let userPannedOverview = false;
    let isProgrammaticMove = false;

    function fitOverview() {
      if (destroyed || !lastOverviewBounds || userPannedOverview) return;
      const llBounds = geoJsonBoundsToLeaflet(lastOverviewBounds);
      if (llBounds) {
        const padding = computeFitPadding();
        isProgrammaticMove = true;
        try {
          const bounds = L.latLngBounds(llBounds);
          const topLeft = L.point(padding.paddingTopLeft);
          const bottomRight = L.point(padding.paddingBottomRight);
          const zoom = Math.max(8, map.getBoundsZoom(bounds, false, topLeft.add(bottomRight)));
          // Apply sheet padding at the final zoom, so mobile framing stays over Texas.
          const center = map.project(bounds.getCenter(), zoom).add(bottomRight.subtract(topLeft).divideBy(2));
          map.setView(map.unproject(center, zoom), zoom, { animate: false });
        } finally {
          queueMicrotask(() => { isProgrammaticMove = false; });
        }
      }
    }

    // Initial bounds fitting
    fitOverview();
    let overviewFitTimer = setTimeout(() => {
      overviewFitTimer = null;
      if (!destroyed && !userPannedOverview && selectionLayer === null) {
        fitOverview();
      }
    }, 80);

    // Layer groups
    const overviewGroup = L.featureGroup().addTo(map);
    const overviewMap = new Map();

    const userLocationGroup = L.featureGroup().addTo(map);

    let selectionLayer = null;
    let startMarker = null;
    let selectedKey = null;
    let lastSelectionBounds = null;
    let userPannedSelection = false;

    // Marker cluster group for places
    let placesGroup = null;
    if (typeof L.markerClusterGroup === 'function') {
      placesGroup = L.markerClusterGroup({
        maxClusterRadius: 40,
        showCoverageOnHover: false,
        spiderfyOnMaxZoom: true,
        chunkedLoading: true
      });
    } else {
      placesGroup = L.featureGroup();
    }
    placesGroup.addTo(map);

    // Map locate control button
    const LocateControl = L.Control.extend({
      options: { position: 'topleft' },
      onAdd: function () {
        const btn = L.DomUtil.create('button', 'leaflet-bar leaflet-control leaflet-control-locate');
        btn.setAttribute('type', 'button');
        btn.setAttribute('title', 'Locate me on map');
        btn.setAttribute('aria-label', 'Locate me on map');
        btn.innerHTML = '<span class="locate-icon" aria-hidden="true">📍</span>';
        L.DomEvent.disableClickPropagation(btn);
        L.DomEvent.on(btn, 'click', function (e) {
          L.DomEvent.preventDefault(e);
          adapter.locateUser();
        });
        return btn;
      }
    });
    new LocateControl().addTo(map);

    let hillCountryBtn = null;
    function syncHillCountryButton() {
      if (!hillCountryBtn) return;
      const active = map.hasLayer(hillCountryLayer);
      hillCountryBtn.setAttribute('aria-pressed', active ? 'true' : 'false');
      hillCountryBtn.classList.toggle('is-active', active);
    }

    const HillCountryControl = L.Control.extend({
      options: { position: 'topleft' },
      onAdd: function () {
        const btn = L.DomUtil.create('button', 'leaflet-bar leaflet-control leaflet-control-hill-country is-active');
        btn.setAttribute('type', 'button');
        btn.setAttribute('title', 'Toggle Texas Hill Country boundary');
        btn.setAttribute('aria-label', 'Toggle Texas Hill Country boundary');
        btn.setAttribute('aria-pressed', 'true');
        btn.innerHTML = '<span class="hill-country-swatch" aria-hidden="true"></span><span>Hill Country</span>';
        L.DomEvent.disableClickPropagation(btn);
        L.DomEvent.on(btn, 'click', function (e) {
          L.DomEvent.preventDefault(e);
          if (map.hasLayer(hillCountryLayer)) {
            map.removeLayer(hillCountryLayer);
          } else {
            hillCountryLayer.addTo(map);
          }
          syncHillCountryButton();
        });
        hillCountryBtn = btn;
        syncHillCountryButton();
        return btn;
      }
    });
    new HillCountryControl().addTo(map);
    map.on('overlayadd', syncHillCountryButton);
    map.on('overlayremove', syncHillCountryButton);

    // Moveend listener for boundsChanged
    function publishBounds(initial) {
      if (destroyed) return;
      if (typeof config.onBoundsChanged === 'function') {
        const bounds = leafletBoundsToGeoJson(map.getBounds());
        config.onBoundsChanged(bounds, initial);
      }
    }

    function noteUserMapInteraction() {
      if (!isProgrammaticMove) {
        if (overviewFitTimer) {
          clearTimeout(overviewFitTimer);
          overviewFitTimer = null;
        }
        if (adapter.hasSelection()) {
          userPannedSelection = true;
        } else {
          userPannedOverview = true;
        }
      }
    }

    function handleMoveEnd() {
      const programmatic = isProgrammaticMove;
      isProgrammaticMove = false;
      if (!programmatic) {
        if (overviewFitTimer) {
          clearTimeout(overviewFitTimer);
          overviewFitTimer = null;
        }
        if (adapter.hasSelection()) {
          userPannedSelection = true;
        } else {
          userPannedOverview = true;
        }
      }
      publishBounds(programmatic);
    }
    function handleResize() { publishBounds(true); }
    map.on('moveend', handleMoveEnd);
    map.on('resize', handleResize);
    map.on('movestart', noteUserMapInteraction);
    map.on('dragstart', noteUserMapInteraction);
    map.on('zoomstart', noteUserMapInteraction);

    function computeFitPadding() {
      const padTop = 24;
      const padLeft = 24;
      const padRight = 24;
      let padBottom = 24;

      if (window.innerWidth < 768) {
        const bottomPanel = document.querySelector('.bottom-panel');
        let panelHeight = 0;
        if (bottomPanel) {
          if (bottomPanel.classList.contains('snap-expanded')) {
            panelHeight = window.innerHeight * 0.58;
          } else {
            const rect = bottomPanel.getBoundingClientRect();
            panelHeight = rect.height;
            if (panelHeight <= 0) {
              if (bottomPanel.classList.contains('snap-collapsed')) {
                panelHeight = window.innerHeight * 0.20;
              } else {
                panelHeight = window.innerHeight * 0.58;
              }
            }
          }
        } else {
          panelHeight = window.innerHeight * 0.58;
        }
        const maxAllowedPadding = Math.max(24, window.innerHeight - 100);
        padBottom = Math.min(Math.round(panelHeight) + 16, maxAllowedPadding);
      }

      return {
        paddingTopLeft: [padLeft, padTop],
        paddingBottomRight: [padRight, padBottom]
      };
    }

    // Adapter instance
    const adapter = {
      map: map,
      hillCountryLayer: hillCountryLayer,

      setOverview: function (key, feature) {
        if (destroyed || !key || !feature) return;
        const kStr = keyToString(key);

        if (overviewMap.has(kStr)) {
          overviewGroup.removeLayer(overviewMap.get(kStr));
          overviewMap.delete(kStr);
        }

        const color = (feature.properties && feature.properties.color)
          ? feature.properties.color
          : '#2563eb';

        const layer = L.geoJSON(feature, {
          renderer: canvasRenderer,
          style: {
            color: color,
            weight: 2.5,
            opacity: selectionLayer ? 0.18 : 0.55
          }
        });

        // Click selection on line
        layer.on('click', function (e) {
          if (destroyed) return;
          if (e && e.originalEvent) {
            L.DomEvent.stopPropagation(e.originalEvent);
          }
          if (typeof config.onSelect === 'function') {
            config.onSelect(key);
          }
        });

        // Popup with safe textContent
        if (feature.properties && feature.properties.title) {
          const titleDiv = document.createElement('div');
          titleDiv.className = 'map-popup-title';
          titleDiv.textContent = feature.properties.title;
          layer.bindPopup(titleDiv);
          const label = document.createElement('span');
          label.textContent = feature.properties.title;
          layer.bindTooltip(label, { sticky: true, className: 'route-name-label' });
        }

        layer.on('mouseover', () => { layer.setStyle({ weight: 5, opacity: 1 }); layer.bringToFront(); });
        layer.on('mouseout', () => {
          layer.setStyle({ weight: 2.5, opacity: selectionLayer ? 0.18 : 0.55 });
          if (selectionLayer && selectionLayer.bringToFront) selectionLayer.bringToFront();
        });
        overviewGroup.addLayer(layer);
        overviewMap.set(kStr, layer);
      },

      removeOverview: function (key) {
        if (destroyed || !key) return;
        const kStr = keyToString(key);
        if (overviewMap.has(kStr)) {
          const layer = overviewMap.get(kStr);
          overviewGroup.removeLayer(layer);
          overviewMap.delete(kStr);
        }
      },

      setPlaces: function (points) {
        if (destroyed) return;
        placesGroup.clearLayers();
        if (!Array.isArray(points)) return;

        points.forEach(function (pt) {
          if (!pt || !pt.coordinates || pt.coordinates.length < 2) return;
          const lat = pt.coordinates[1];
          const lon = pt.coordinates[0];

          const marker = L.marker([lat, lon]);

          if (pt.title) {
            const popupDiv = document.createElement('div');
            popupDiv.className = 'place-popup';
            const titleEl = document.createElement('strong');
            titleEl.textContent = pt.title;
            popupDiv.appendChild(titleEl);
            marker.bindPopup(popupDiv);
          }

          marker.on('click', function (e) {
            if (destroyed) return;
            if (e && e.originalEvent) {
              L.DomEvent.stopPropagation(e.originalEvent);
            }
            if (typeof config.onSelect === 'function') {
              config.onSelect(pt.key);
            }
          });

          placesGroup.addLayer(marker);
        });
      },

      setSelection: function (key, feature) {
        if (destroyed) return;
        this.clearSelection();
        if (!key || !feature) return;

        userPannedSelection = false;
        selectedKey = key;

        if (feature.geometry && feature.geometry.type === 'Point') {
          const coords = feature.geometry.coordinates;
          const lat = coords[1];
          const lon = coords[0];

          selectionLayer = L.circleMarker([lat, lon], {
            radius: 12,
            color: '#f59e0b',
            fillColor: '#fbbf24',
            fillOpacity: 0.9,
            weight: 3
          }).addTo(map);
        } else {
          // LineString / MultiLineString
          const color = (feature.properties && feature.properties.color)
            ? feature.properties.color
            : '#2563eb';

          selectionLayer = L.geoJSON(feature, {
            renderer: canvasRenderer,
            style: {
              color: color,
              weight: 6,
              opacity: 1.0
            }
          }).addTo(map);

          let firstCoord = null;
          if (feature.geometry) {
            if (feature.geometry.type === 'LineString' && Array.isArray(feature.geometry.coordinates) && feature.geometry.coordinates.length > 0) {
              firstCoord = feature.geometry.coordinates[0];
            } else if (feature.geometry.type === 'MultiLineString' && Array.isArray(feature.geometry.coordinates) && feature.geometry.coordinates.length > 0 && Array.isArray(feature.geometry.coordinates[0]) && feature.geometry.coordinates[0].length > 0) {
              firstCoord = feature.geometry.coordinates[0][0];
            }
          }

          if (firstCoord && firstCoord.length >= 2) {
            const startLat = firstCoord[1];
            const startLon = firstCoord[0];
            const startIcon = L.divIcon({
              className: 'route-start-marker-container',
              html: '<span class="route-start-marker" aria-label="Route start">Start</span>',
              iconSize: [44, 22],
              iconAnchor: [22, 11]
            });
            startMarker = L.marker([startLat, startLon], {
              icon: startIcon,
              title: 'Start: ' + (feature.properties && feature.properties.title ? feature.properties.title : 'Route start'),
              keyboard: false
            }).addTo(map);
          }
        }

        if (feature.properties && feature.properties.title) {
          const popupDiv = document.createElement('div');
          popupDiv.className = 'selection-popup';
          popupDiv.textContent = feature.properties.title;
          selectionLayer.bindPopup(popupDiv);
          if (feature.geometry && feature.geometry.type !== 'Point') {
            const label = document.createElement('span');
            label.textContent = feature.properties.title;
            selectionLayer.bindTooltip(label, { permanent: true, direction: 'center', className: 'route-name-label' }).openTooltip();
          }
        }
        overviewMap.forEach(layer => layer.setStyle({ weight: 2.5, opacity: 0.18 }));
      },

      hasSelection: function () {
        return selectionLayer !== null;
      },

      getSelectedKey: function () {
        return selectedKey;
      },

      getSelectionPopupContent: function () {
        if (!selectionLayer) return null;
        const popup = selectionLayer.getPopup ? selectionLayer.getPopup() : null;
        if (!popup) return null;
        const content = popup.getContent();
        if (typeof content === 'string') return content;
        if (content && content.textContent) return content.textContent;
        return null;
      },

      clearSelection: function () {
        if (selectionLayer) {
          map.removeLayer(selectionLayer);
          selectionLayer = null;
        }
        if (startMarker) {
          map.removeLayer(startMarker);
          startMarker = null;
        }
        overviewMap.forEach(layer => layer.setStyle({ weight: 2.5, opacity: 0.55 }));
        selectedKey = null;
        lastSelectionBounds = null;
        userPannedSelection = false;
      },

      fitBounds: function (bounds) {
        if (destroyed || !bounds) return;
        if (selectedKey !== null) {
          lastSelectionBounds = bounds;
        }
        const llBounds = geoJsonBoundsToLeaflet(bounds);
        if (llBounds) {
          const padding = computeFitPadding();
          isProgrammaticMove = true;
          try {
            map.fitBounds(llBounds, {
              paddingTopLeft: padding.paddingTopLeft,
              paddingBottomRight: padding.paddingBottomRight,
              animate: false
            });
          } finally {
            queueMicrotask(() => { isProgrammaticMove = false; });
          }
        }
      },

      reframeSelection: function () {
        if (destroyed || !this.hasSelection() || !lastSelectionBounds) return;
        userPannedSelection = false;
        this.fitBounds(lastSelectionBounds);
      },

      reframeOverview: function () {
        if (destroyed || this.hasSelection() || !lastOverviewBounds) return;
        const bottomPanel = document.querySelector('.bottom-panel');
        if (bottomPanel && bottomPanel.classList.contains('snap-expanded')) return;
        userPannedOverview = false;
        fitOverview();
      },

      locateUser: function (onSuccess, onError) {
        if (destroyed) return;
        if (typeof navigator === 'undefined' || !navigator.geolocation) {
          const msg = 'Geolocation is not supported by your browser';
          if (typeof onError === 'function') onError(msg);
          if (typeof config.onLocationError === 'function') config.onLocationError(msg);
          return;
        }

        navigator.geolocation.getCurrentPosition(
          function (pos) {
            if (destroyed) return;
            const lat = pos.coords.latitude;
            const lon = pos.coords.longitude;
            const accuracy = pos.coords.accuracy || 0;
            adapter.setUserLocation(lat, lon, accuracy);
            if (typeof onSuccess === 'function') onSuccess(lat, lon, accuracy);
            if (typeof config.onUserLocation === 'function') config.onUserLocation(lat, lon, accuracy);
          },
          function (err) {
            if (destroyed) return;
            let msg = 'Unable to retrieve your location';
            if (err.code === 1) msg = 'Location access was denied. Please allow location permissions to find routes near you.';
            else if (err.code === 2) msg = 'Location information is currently unavailable.';
            else if (err.code === 3) msg = 'Location request timed out. Please try again.';
            if (typeof onError === 'function') onError(msg);
            if (typeof config.onLocationError === 'function') config.onLocationError(msg);
          },
          { enableHighAccuracy: true, timeout: 10000, maximumAge: 60000 }
        );
      },

      setUserLocation: function (lat, lon, accuracy) {
        if (destroyed) return;
        userLocationGroup.clearLayers();

        if (typeof accuracy === 'number' && accuracy > 0) {
          // Share the route renderer so the accuracy overlay cannot cover its canvas.
          L.circle([lat, lon], {
            renderer: canvasRenderer,
            radius: Math.min(accuracy, 8000),
            color: '#2563eb',
            weight: 1,
            fillColor: '#3b82f6',
            fillOpacity: 0.15,
            interactive: false
          }).addTo(userLocationGroup);
        }

        const pulseHtml = '<div class="user-pulse-marker"><div class="user-pulse-ring"></div><div class="user-pulse-dot"></div></div>';
        const userIcon = L.divIcon({
          className: 'user-location-icon-container',
          html: pulseHtml,
          iconSize: [24, 24],
          iconAnchor: [12, 12]
        });

        // Location is an indicator: let clicks reach routes beneath the pin.
        L.marker([lat, lon], {
          icon: userIcon,
          interactive: false,
          keyboard: false,
          zIndexOffset: 1000
        }).addTo(userLocationGroup);

        const currentZoom = map.getZoom();
        map.setView([lat, lon], currentZoom < 10 ? 11 : currentZoom);
      },

      clearUserLocation: function () {
        if (destroyed) return;
        userLocationGroup.clearLayers();
      },

      resize: function () {
        if (destroyed) return;
        map.invalidateSize();
      },

      destroy: function () {
        if (destroyed) return;
        destroyed = true;

        if (overviewFitTimer) {
          clearTimeout(overviewFitTimer);
          overviewFitTimer = null;
        }

        map.off('moveend', handleMoveEnd);
        map.off('resize', handleResize);
        map.off('movestart', noteUserMapInteraction);
        map.off('dragstart', noteUserMapInteraction);
        map.off('zoomstart', noteUserMapInteraction);
        map.off('overlayadd', syncHillCountryButton);
        map.off('overlayremove', syncHillCountryButton);

        this.clearSelection();

        overviewMap.forEach(function (layer) {
          overviewGroup.removeLayer(layer);
        });
        overviewMap.clear();

        userLocationGroup.clearLayers();
        map.removeLayer(userLocationGroup);

        placesGroup.clearLayers();
        map.removeLayer(placesGroup);
        map.removeLayer(overviewGroup);
        hillCountryLayer.clearLayers();
        map.removeLayer(hillCountryLayer);

        document.removeEventListener('transitionend', handleTransitionEnd);

        map.remove();

        container._rideAtlasAdapter = null;
        container._leaflet_id = null;
      }
    };

    function handleTransitionEnd(e) {
      if (destroyed) return;
      if (e.target && e.target.classList && e.target.classList.contains('bottom-panel')) {
        if (e.propertyName === 'height') {
          if (e.target.classList.contains('snap-expanded')) {
            return;
          }
          if (adapter.hasSelection() && lastSelectionBounds && !userPannedSelection) {
            adapter.fitBounds(lastSelectionBounds);
          } else if (!adapter.hasSelection() && lastOverviewBounds && !userPannedOverview) {
            fitOverview();
          }
        }
      }
    }
    document.addEventListener('transitionend', handleTransitionEnd);

    container._rideAtlasAdapter = adapter;
    queueMicrotask(() => publishBounds(true));
    return adapter;
  }

  return {
    createMap: createMap
  };
});
