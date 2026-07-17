# <div align="center">⭐️ Spot-MP ⭐️</div>

---

#### Gráfico Representativo del Reproductor General:

#### Casos de Uso Principales (Base):

| Caso de uso: Toggle Play/Pause |
| ------ |
| **ID:** 31 |
| **Breve descripción:** <br> El Usuario pausa o reanuda la canción que se encuentra actualmente cargada en el reproductor.|
| **Actores principales:** Usuario |
| **Actores secundarios:** Ninguno |
| **Precondiciones:** <br> 1. Existe una canción actualmente cargada o reproduciéndose en el **motor de audio**. |
| **Flujo principal:** <br> 1. El caso de uso empieza cuando el Usuario acciona el control de pausa/reproducción. <br> 2. El sistema verifica el estado actual del audio. <br> 3. El sistema invierte el estado (si está reproduciendo, lo pausa; si está pausado, lo reanuda).<br>4. El sistema actualiza la interfaz visual con el estado actual.|
| **Postcondiciones:** <br> 1. El estado de la reproducción se ha alternado exitosamente. |
| **Flujos alternativos:** <br> Ninguno. |

| Caso de uso: Seek Position (Controlar momento) |
| ------ |
| **ID:** 32 |
| **Breve descripción:** <br> El Usuario modifica el momento exacto de reproducción de la canción actual utilizando un slider interactivo.|
| **Actores principales:** Usuario |
| **Actores secundarios:** Ninguno |
| **Precondiciones:** <br> 1. Hay una canción cargada en el reproductor. |
| **Flujo principal:** <br> 1. El caso de uso empieza cuando el Usuario arrastra o hace clic en un punto específico del slider de progreso.<br> 2. El sistema calcula la marca de tiempo correspondiente a la posición seleccionada. <br> 3. El sistema ordena al motor de audio saltar a dicha marca de tiempo. |
| **Postcondiciones:** <br> 1. La canción continúa reproduciéndose (o pausada) desde el nuevo punto seleccionado. |
| **Flujos alternativos:** <br> Ninguno. |

| Caso de uso: Change Playback Flow |
| ------ |
| **ID:** 33 |
| **Breve descripción:** <br> El Usuario alterna entre los distintos modos de flujo de reproducción: Normal, Random (Aleatorio) o En Ciclo (Repetición).|
| **Actores principales:** Usuario |
| **Actores secundarios:** Ninguno |
| **Precondiciones:** <br> 1. El reproductor se encuentra activo. |
| **Flujo principal:** <br> 1. El caso de uso empieza cuando el Usuario interactúa con el botón de control de flujo. <br> 2. El sistema cambia al siguiente estado disponible (Ej: Normal -> Random -> En Ciclo).<br> 3. El sistema reconfigura la lista de próximas canciones basándose en el nuevo estado (ej. generando una lista enlazada aleatoria si se activa Random).|
| **Postcondiciones:** <br> 1. El motor de reproducción actualiza sus reglas para determinar cuál será la siguiente canción. |
| **Flujos alternativos:** <br> Ninguno. |

| Caso de uso: Skip to Previous Song |
| ------ |
| **ID:** 34 |
| **Breve descripción:** <br> El sistema retrocede a la canción anterior basándose en la lista doblemente enlazada de reproducción, desencadenado por acción manual del Usuario.|
| **Actores principales:** Usuario. |
| **Actores secundarios:** Ninguno |
| **Precondiciones:** <br> 1. El Usuario acciona el botón de "Anterior". |
| **Flujo principal:** <br> 1. El sistema evalúa si existen canciones anteriores en la **Lista de reproducción.**<br> 2. Si hay canciones en la lista, el sistema extrae y la reproduce.<br> 3. Si el sistema se encuentra en la primer cancion de la lista de reproducción no hace nada.|
| **Postcondiciones:** <br> 1. El reproductor carga e inicia la pista anterior correspondiente, o sigue con su reproducción normalmente de la canción actual. |
| **Flujos alternativos:** <br> Ninguno. |

| Caso de uso: Skip to Next Song |
| ------ |
| **ID:** 35 |
| **Breve descripción:** <br> El sistema avanza a la siguiente canción basándose en un flujo de prioridades estricto, ya sea de forma automática al terminar la actual o por acción manual del Usuario.|
| **Actores principales:** Usuario (si es manual), Sistema (si es automático) |
| **Actores secundarios:** Ninguno |
| **Precondiciones:** <br> 1. Una canción finaliza su tiempo de duración, o el Usuario acciona el botón de "Siguiente". |
| **Flujo principal:** <br> 1. El sistema evalúa si existen canciones pendientes en la **Cola de reproducción** (prioridad más alta).<br> 2. Si hay canciones en la cola, el sistema extrae y reproduce la primera.<br> 3. Si la cola está vacía, el sistema verifica si el flujo activo es "En Ciclo"; de ser así, repite la canción actual.<br> 4. Si el flujo es Normal o Random, y se está reproduciendo desde una playlist, el sistema reproduce la siguiente canción de la lista (ordenada o aleatoria según corresponda).<br> 5. Si la reproducción provenía de una canción simple (sin cola ni ciclo), el reproductor se pausa.|
| **Postcondiciones:** <br> 1. El reproductor carga e inicia la siguiente pista correspondiente, o se detiene si no hay más elementos en el flujo. |
| **Flujos alternativos:** <br> Ninguno. |

| Caso de uso: Add to Playlist (Desde Reproductor) |
| ------ |
| **ID:** 36 |
| **Breve descripción:** <br> El Usuario decide agregar la canción que está sonando actualmente a una playlist mediante el control del reproductor general. La acción delega el comportamiento a la vista activa.|
| **Actores principales:** Usuario |
| **Actores secundarios:** Ninguno (Dependera de la página) |
| **Precondiciones:** <br> 1. Hay una canción cargada en el reproductor general. |
| **Flujo principal:** <br> 1. El Usuario acciona el botón de agregar a playlist ubicado en la interfaz del reproductor general. <br> 2. El sistema detecta la página actual (Local, Playlist o Spotify) y delega la acción a su respectivo manejador de listas (ej. ShowLocalPlaylists o Show Playlists). <br> 3. El menú de listas correspondiente se despliega en pantalla. <br> 4. El Usuario selecciona la lista destino.|
| **Postcondiciones:** <br> 1. La canción en curso queda registrada en las playlists seleccionadas por el Usuario. |
| **Flujos alternativos:** <br>Ninguno |