import operator
from copy import copy

from build123d import *
import ocp_vscode

MODULE_SIZE = 40
SCREW_DIA = 2.0 + 0.2
SCREW_TO_EDGES = 2.2
EDGE_FILLET_R = SCREW_TO_EDGES
MAIN_PCB_THICKNESS = 1.2
PERI_PCB_THICKNESS = 1.2

MAGNET_TO_SURFACE = 0.4
MAGNET_WIDTH = 5.0
MAGNET_HEIGHT = 5.0
MAGNET_THICKNESS = 5.0
MAGNET_HORIZONTAL_DISTANCE = 29.0

magnet = Part(Box(
    MAGNET_THICKNESS, MAGNET_WIDTH, MAGNET_HEIGHT,
    align=(Align.MIN, Align.CENTER, Align.MIN)
), label="magnet", color="gray")

magnet_locations = [
    Rot(Z = i // 2 * 90) * Pos(
        -(MODULE_SIZE / 2 - MAGNET_TO_SURFACE),
        (MAGNET_HORIZONTAL_DISTANCE - MAGNET_WIDTH) / 2 * ((-1) ** i)
    )
    for i in range(8)
]

magnets = Compound(label="magnets", children=[copy(magnet).locate(l) for l in magnet_locations])

@operator.call
def bottom_part():
    with BuildPart() as bottom_part:
        BATT_HOLDER_D = 29.5
        BATT_HOLDER_HEIGHT = 8.6
        BOTTOM_PART_THICKNESS = 9.0

        with BuildSketch():
            RectangleRounded(
                MODULE_SIZE, MODULE_SIZE,
                EDGE_FILLET_R
            )
        extrude(amount=BOTTOM_PART_THICKNESS)

        with Locations(bottom_part.faces().sort_by(Axis.Z)[0]):
            with GridLocations(*([MODULE_SIZE - SCREW_TO_EDGES * 2] * 2), 2, 2):
                CounterSinkHole(
                    SCREW_DIA / 2,
                    counter_sink_radius=4.2 / 2,
                    counter_sink_angle=82
                )

        with Locations(bottom_part.faces().sort_by(Axis.Z)[-1]):
            Hole(BATT_HOLDER_D / 2)
            Hole(35.0 / 2, 5.0)
            [Box(
                MODULE_SIZE - 0.8 * 2, MODULE_SIZE - SCREW_TO_EDGES * 4 - 2.0, 2.0,
                rotation=(0, 0, rot),
                align=(Align.CENTER, Align.CENTER, Align.MAX),
                mode=Mode.SUBTRACT
            ) for rot in [0.0, 90.0]]
        
        with Locations(Pos(Z=1.0)):
            with Locations(magnet_locations):
                Box(
                    MAGNET_THICKNESS + 0.1, MAGNET_WIDTH + 0.1, bottom_part.max_dimension,
                    align=(Align.MIN, Align.CENTER, Align.MIN),
                    mode=Mode.SUBTRACT
                )
        
        # SWD connector
        with Locations(Pos(Y=-MODULE_SIZE / 2, Z=BOTTOM_PART_THICKNESS)):
            Box(
                5.0 + 2.0 + 0.2, 5.0, 3.1 + 0.1,
                align=(Align.CENTER, Align.MIN, Align.MAX),
                mode=Mode.SUBTRACT
            )

    bottom_part.part.label = "bottom_part"
    bottom_part.part.color = "black"
    return bottom_part.part

bottom_assembly = Compound(
    label="bottom_assembly",
    children=[
        bottom_part,
        magnets.moved(Pos(Z=1.0))
    ]
)

def pcb_base(thickness: float):
    with BuildPart(mode=Mode.PRIVATE) as pcb_base:
        with BuildSketch():
            RectangleRounded(
                MODULE_SIZE, MODULE_SIZE,
                EDGE_FILLET_R
            )
            with GridLocations(*([MODULE_SIZE - SCREW_TO_EDGES * 2] * 2), 2, 2):
                Circle(2.2 / 2, mode=Mode.SUBTRACT)
        extrude(amount=thickness)

    pcb_base.part.label = "pcb_base"
    pcb_base.part.color = "green"
    return pcb_base.part

main_pcb = pcb_base(MAIN_PCB_THICKNESS)

@operator.call
def batt_holder():
    with BuildPart(main_pcb.faces().sort_by(Axis.Z)[0]) as batt_holder:
        Cylinder(
            29.5 / 2,
            1.5,
            align=(Align.CENTER, Align.CENTER, Align.MIN)
        )
        Cylinder(
            (29.5 - 1.0) / 2,
            7.6,
            align=(Align.CENTER, Align.CENTER, Align.MIN)
        )
        Box(
            30.5, 6.95, 4.7,
            align=(Align.CENTER, Align.CENTER, Align.MIN)
        )
        Box(
            33.5, 3.6, 0.5,
            align=(Align.CENTER, Align.CENTER, Align.MIN)
        )

    batt_holder.part.label = "batt_holder"
    batt_holder.part.color = "chocolate"
    return batt_holder.part

CONNECTORS_CENTER_OFFSET = 8.0

@operator.call
def male_connector():
    with BuildPart() as part:
        Box(
            4.0, 14.0, 3.8,
            align=(Align.MIN, Align.CENTER, Align.MIN)
        ).move(Pos(0, 0, 0.1))

    part.part.label = "female_connector"
    part.part.color = "teal"

    return part.part

@operator.call
def female_connector():
    with BuildPart() as part:
        Box(1.6, 12.5, 6.0, align=(Align.MIN, Align.CENTER, Align.MIN))
        with BuildSketch(part.faces().sort_by(Axis.X)[-1].location_at(0, 0.5)):
            with Locations((6.0 - 4.3, 7.5 / 2)):
                Circle(1.1 / 2)
            mirror(about=Plane.XZ)
        extrude(amount=1.0)

    part.part.label = "female_connector"
    part.part.color = "orange"
    return part.part

male_connector_locations = [
    Rot(Z = i * 90) * Pos(-MODULE_SIZE / 2, CONNECTORS_CENTER_OFFSET)
    for i in range(4)
]
female_connector_locations = [
    Rot(Z = i * 90) * Pos(-MODULE_SIZE / 2, -CONNECTORS_CENTER_OFFSET)
    for i in range(4)
]

main_pcba = Compound(
    label="main_pcba",
    children=[
        main_pcb,
        batt_holder.rotate(Axis.Z, 45),
        Compound(label="Connectors", children=[
            *[male_connector.located(l) for l in male_connector_locations],
            *[female_connector.located(l) for l in female_connector_locations],
        ]).move(Pos(Z=MAIN_PCB_THICKNESS))
    ]
).locate(Location((0, 0, bottom_part.bounding_box().max.Z)))

MIDDLE_PART_THICKNESS = 11.0
MIDDLE_PART_WALL = 5.0

LCD_SIZE_X = 31.52
LCD_SIZE_Y = 33.72
LCD_ACTIVE = 27.72
LCD_EDGE_TOP = 1.45
LCD_THICKNESS = 1.9
LCD_TO_PCB = 4.3 + 0.2
LCD_FPC_WIDTH = 22.0
LCD_FPC_SMALL_WIDTH = 6.5

@operator.call
def middle_part():
    with BuildPart() as middle_part:        
        with BuildSketch():
            RectangleRounded(
                MODULE_SIZE, MODULE_SIZE,
                EDGE_FILLET_R
            )
            Rectangle(
                *([MODULE_SIZE - MIDDLE_PART_WALL * 2] * 2),
                mode=Mode.SUBTRACT
            )
        extrude(amount=MIDDLE_PART_THICKNESS)

        # extra clearance close to PCBs
        mirror(
            Box(
                *([MODULE_SIZE - 4.0 * 2] * 2),
                2.0,
                mode=Mode.SUBTRACT
            ),
            about=Plane.XY.offset(MIDDLE_PART_THICKNESS / 2),
            mode=Mode.SUBTRACT
        )

        # screws
        with GridLocations(*([MODULE_SIZE - SCREW_TO_EDGES * 2] * 2), 2, 2):
            mirror( # heat inserts
                Hole(3.0 / 2, 4.0),
                about=middle_part.workplanes[0].offset(MIDDLE_PART_THICKNESS / 2),
                mode=Mode.SUBTRACT
            )
            Hole(SCREW_DIA / 2)
        
        # magnets
        with Locations(Pos(Z=MIDDLE_PART_THICKNESS)):
            with Locations(magnet_locations):
                Box(
                    MAGNET_THICKNESS * 2 + 0.1, MAGNET_WIDTH + 0.1, MAGNET_HEIGHT + 0.1,
                    align=(Align.MIN, Align.CENTER, Align.MAX),
                    mode=Mode.SUBTRACT
                )
        
        # USB port
        with Locations(Pos(X=MODULE_SIZE / 2, Z=MIDDLE_PART_THICKNESS)):
            Box(
                8.0, 8.94 + 0.2, 3.16 + 0.1,
                align=(Align.MAX, Align.CENTER, Align.MAX),
                mode=Mode.SUBTRACT
            )
        
        # LCD FPC
        with Locations(Pos(Y=-MODULE_SIZE / 2 + 0.8, Z=MIDDLE_PART_THICKNESS)):
            Box(
                LCD_FPC_SMALL_WIDTH + 1.0, MODULE_SIZE / 2, 2.0,
                align=(Align.CENTER, Align.MIN, Align.MAX),
                mode=Mode.SUBTRACT
            )

        with Locations(male_connector_locations):
            Box(
                4.0 + 0.2, 14.0 + 0.4, 3.9 + 0.2,
                align=(Align.MIN, Align.CENTER, Align.MIN),
                mode=Mode.SUBTRACT
            )
            with Locations(Pos(X=0.4)):
                Box(
                    4.0 - 0.4, 14 + 0.8 * 2 + 0.2, 1.0,
                    align=(Align.MIN, Align.CENTER, Align.MIN),
                    mode=Mode.SUBTRACT
                )
        
        with BuildPart(*female_connector_locations, mode=Mode.SUBTRACT):
            Box(
                1.6 + 0.2, 12.5 + 0.4, 6.0 + 0.2,
                align=(Align.MIN, Align.CENTER, Align.MIN)
            )
            with Locations(*[Pos(Y=7.5 / 2 * m) for m in [-1, 1]]):
                Box(
                    1.6 + 1.0 + 0.6, 1.1 + 0.4, 6.0 - 4.3 + 1.1 / 2 + 0.2,
                    align=(Align.MIN, Align.CENTER, Align.MIN)
                )

    middle_part.part.label = "middle_part"
    middle_part.part.color = "black"
    return middle_part.part

middle_assembly = Compound(label="middle_assembly", children=[
    middle_part,
    magnets.moved(Pos(Z=MIDDLE_PART_THICKNESS - MAGNET_HEIGHT))
]).locate(main_pcb.global_location * Plane(main_pcb.faces().sort_by(Axis.Z)[-1]).location)

peri_pcb = pcb_base(PERI_PCB_THICKNESS)

lcd = Part(Box(
    LCD_SIZE_X, LCD_SIZE_Y, LCD_THICKNESS,
    align=(Align.CENTER, Align.MAX, Align.MIN)
), label="lcd", color="white") \
    .locate(Plane(peri_pcb.faces().sort_by(Axis.Z)[-1]).location) \
    .move(Pos(Y=LCD_EDGE_TOP + LCD_ACTIVE / 2, Z=LCD_TO_PCB))

peri_pcba = Compound(label="peri_pcba", children=[peri_pcb, lcd]) \
    .locate(middle_part.global_location * Pos(Z=middle_part.bounding_box().max.Z))

@operator.call
def top_part():
    with BuildPart(peri_pcb.global_location * Plane(peri_pcb.faces().sort_by(Axis.Z)[-1]).location) as top_part:
        LCD_COVER_THICKNESS = 0.6

        with BuildSketch(*top_part.workplanes):
            RectangleRounded(
                MODULE_SIZE, MODULE_SIZE,
                EDGE_FILLET_R
            )
        _total_thickness = LCD_TO_PCB + LCD_THICKNESS + LCD_COVER_THICKNESS
        extrude(amount = _total_thickness)

        with GridLocations(*([MODULE_SIZE - SCREW_TO_EDGES * 2] * 2), 2, 2):
            Hole(SCREW_DIA / 2)
        
        # main LCD slot
        with Locations(Pos(Y=LCD_EDGE_TOP + LCD_ACTIVE / 2 + 0.3)):
            Box(
                LCD_SIZE_X + 0.6, LCD_SIZE_Y + 0.6, LCD_TO_PCB + LCD_THICKNESS,
                align=(Align.CENTER, Align.MAX, Align.MIN),
                mode=Mode.SUBTRACT
            )
        
        # LCD FPC slot
        with Locations(Pos(Z=_total_thickness - LCD_COVER_THICKNESS)):
            Box(
                LCD_FPC_WIDTH + 1.0,
                LCD_SIZE_Y - LCD_EDGE_TOP - LCD_ACTIVE / 2 + 0.8,
                LCD_THICKNESS + 0.5,
                align=(Align.CENTER, Align.MAX, Align.MAX),
                mode=Mode.SUBTRACT
            )
        
        # LCD viewing window
        with Locations(Pos(Z=_total_thickness)):
            Box(
                LCD_ACTIVE + 2.0, LCD_ACTIVE + 2.0, LCD_COVER_THICKNESS,
                align=(Align.CENTER, Align.CENTER, Align.MAX),
                mode=Mode.SUBTRACT
            )
        
        # buttons clearance
        with Locations([Rot(Z=i) for i in [0, 90]]):
            Box(
                13.5 * 2 + 4.5 + 1.0, 4.5 + 3.0, 4.0 + 0.5,
                align=(Align.CENTER, Align.CENTER, Align.MIN),
                mode=Mode.SUBTRACT
            )
        
        # USB port backside clearance
        with Locations(Pos(X=MODULE_SIZE / 2 - 1.2)):
            Box(
                10.0, 10.0, 2.0,
                align=(Align.MAX, Align.CENTER, Align.MIN),
                mode=Mode.SUBTRACT
            )

    top_part.part.label = "top_part"
    top_part.part.color = "black"
    return top_part.part

print(f"Total thickness: {top_part.bounding_box().max.Z - bottom_assembly.bounding_box().min.Z :.3f}")

ocp_vscode.show(
    top_part,
    peri_pcba,
    middle_assembly,
    main_pcba,
    bottom_assembly,
)

from pathlib import Path
export_path = Path("./export/SnapNodes_RP2")
export_path.mkdir(parents=True, exist_ok=True)
export_stl(bottom_part, export_path / "bottom_part.stl")
export_stl(middle_part, export_path / "middle_part.stl")
export_stl(top_part, export_path / "top_part.stl")
export_step(bottom_part, export_path / "bottom_part.step")
export_step(middle_part, export_path / "middle_part.step")
export_step(top_part, export_path / "top_part.step")