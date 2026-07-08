# <div align="center">⭐️ Spot-MP ⭐️</div>
---
## 💡 Descripcion de la Idea:

### 📂 Musica Local:

&nbsp;Spot-MP debe presentarse como una **aplicación de terminal**, con una interfaz de usuario **interactuable con o sin mouse (shortcut o botones)**. La idea principal es que funcione como un **reproductor de musica local**, este apartado de musica local debe nacer directamente desde una carpeta raiz de musica, **esta raiz de musica debe presentarse al usuario como una pantalla principal**, y las playlist e interacciones con el usuario **se crean y se presentan virtualmente**.

* **¿Que significa esto?**
    &nbsp;El sistema deberá mantener información almacenada de las playlist creadas por el usuario, mostrandolas en una pantalla especial llamada Playlist.

* **Formato del Apartado Local:**
    &nbsp;En el apartado local se debe mostrar los metadatos de la canción si estos existen, como imagen, Titulo, Autor, Nombre de Album y Año de lanzamiento.
* **Funciones para las canciones:**
    &nbsp;En cada canción debe presentar un apartado de botones que permita realizar las siguientes funciones con las canciones:
    * Agregarla a una playlist local (Virtual, no carpetas fisicas).
    * Agregarla a la cola de reproducción.
    * Reproducir la canción directamente.
* **Función de Busqueda:**
    &nbsp;Esta página también presenta un pequeño apartado donde se puede realizar una busqueda por nombre de la canción, esto trabajara con los archivos que se encuentran en la biblioteca raiz, nada online.
* **Shortcut a Playlist:**
    &nbsp;Se presentara un apartado interactuable con la lista de playlist, en caso de querer agregar a una playlist este apartado cambiara a uno con marcadores, marcado significa que la canción ya se encuentra en la playlist, desmarcado es que no.
    &nbsp;Si el usuario interactua con este apartado sin querer agregar una canción a una playlist funcionara como shortcut moviendolo hacia la página de playlist y abriendo el apartado de canciones de la playlist.

---
### 🌐 Conexión con Spotify:
&nbsp;Como se menciona en el nombre **Spot**-MP esta aplicación tambien debera interconectarse con spotify, pero ¿De que manera?. Basicamente la idea principal de interconectarse con spotify es **integrar todas las playlist y canciones que escuchas en tu dia a dia en Spot-MP**, ademas de **presentar una pagina exclusiva para ciertas funcionalidades de spotify**. El principal agregado al interconectar Spot-MP con spotify son siguientes funciones (Ya que esta interconexión es opcional y requiere de una cuenta premium de spotify vinculada):

* **Importar Playlists/Albumes de Spotify:**
    &nbsp;Importar al apartado local de playlist las playlist agregadas en tu perfil de spotify. Estas playlist se diferenciaran con las playlist creadas de manera local, ya que **en estas solo podremos agregar las canciones que pertenezcan a spotify (Se modificaran las playlist en tu perfil de spotify)**.
    &nbsp;En cambio los **Albumes** importados son aquellos likeados en spotify, estos **no son modificables** como las playlist, **hay que hacer cierta diferenciación** entre playlist propias o con permiso de modificación y playlist/Albumes privados que no se nos permite modificar.

* **Buscar Canciones por su Nombre:**
    &nbsp;En la pagina especifica de **Spotify** se presentara un apartado de barra buscadora donde podremos ingresar un nombre y buscar esa canción en spotify mediante el uso de su API.
    * **Apartado Gráfico:** Al buscar una canción por su &nbsp;nombre se debe mostrar una lista de canciones con nombres iguales o parecidos, cada una de ellas con su respectivo conjunto de botones para reproducir, poner en cola o agregar a playlist.
* **Obtener Información de una Canción Especifica (Imagen, Autores, Album, Duración):**
    &nbsp;Debe ser capaz de obtener la información importante de las canciones a reproducir, que se comenta, para presentarlo en playlist locales como en el reproductor.
* **Obtener Información de un Album:**
    &nbsp;Al estar reproduciendo una canción de spotify, se debe mostrar un apartado en la parte derecha de la pagina donde se muestre información sobre el Album, como su Imagen, Autores, fecha de emisión, y una lista de canciones por las cuales podremos navegar, agregar a playlist y reproducirlas.
* **Playlist de Spotify:**
    &nbsp;En esta página se presentara un apartado de playlist que son exclusivamente de Spotify, respetando el diseño de playlist (Arbol con canciones como hojas) y permitiendo al usuario reproducir, agregar a cola y agregar a otras playlists para las canciones dentro de las  mismas.
    &nbsp;Ademas, en la parte superior de esta lista debe presentarse un linea/ListaVacia que represente un boton para crear una nueva lista. Ademas de por cada playlist debe presentarse un botón para su eliminación que desencadenara en la eliminación de la playlist en spotify, para esto se presentara un pequeño cartel de confirmación.
* **Desconectarse de Spotify:**
    &nbsp;Debe presentarse un boton para desloguearte de spotify, esto incluye eliminar las credenciales ingresadas (Client-Id y Client-Secret, Tokens). Para esto se presentara un pequeño cartel de confirmación.

#### Pero Antes de Lograr Todo Eso:

&nbsp;Para realizar una conexión con la API de spotify requerimos un inicio de sesión con diferentes campos y una configuración del **Spotify Developer Dashboard**.
&nbsp;Primeramente al iniciar la aplicación por primera vez, solamente apareceran funcionales las páginas de local y playlist, si el usuario ingresa a la página de Spotify vera un formulario de inicio de sesión con dos campos **Client-Id** y **Client-Secret**. Pero sobre este formulario debera aparecer un aviso que antes de iniciar sesión el usuario debe configurar el **Spotify Developer Dashboard** siguiendo el tutorial explicado en un link que redirecciona al github de Spot-MP donde se explicara paso a paso la configuración.

---
### 📄 Pagina de Playlists:

&nbsp;Antes de presentar los requerimientos de esta página hay que hacer una pequeña diferenciación, una **Playlist Local** es una playlist mixta guardada localmente que se compone tanto de canciones en local como canciones de spotify. En cambio las **Playlist de Spotify** son completamente importadas de spotify y no pueden estar compuestas de canciones locales.

&nbsp;Un resumen general de todos los requerimientos de la pagina de Playlist:

* **Información Mixta:**
    * La pagina debe presentar tanto playlist locales (mixtas) como de spotify.
    * Debe presentarse una diferenciación grafica entre ellas.
* **Apartado Grafico:**
    * Las playlists deben mostrarse con una estructura de arbol donde sus hojas sean las canciones.
    * Las canciones deben mostrar a su izquierda un **boton de play**, seguido de el **nombre de la canción** sobre él el **nombre del album** y **artistas**. Y a su derecha la **duración** de la canción seguido de un **boton de tres puntos** donde se implementen los botones del apartado de modificación.

* **Modificación:**
    * Eliminación de canción de una playlist.
    * Agregar la canción a otra playlist.
    * Poner canción en cola de reproducción.
* **Creación de Playlists:**
    &nbsp;En la parte superior de la lista de playlist se debe presentar una linea que represente la creación de una playlist cuando se aprete, todas las playlist creadas mediante este botón seran playlist mixtas, ya que en la página de playlist habra un apartado especial para crear playlist de spotify.
* **Eliminación de Playlist Locales:**
    &nbsp;Para cada playlist se debe presentar un boton de eliminar playlist, lo que la eliminara de la base de datos, para esto se presentara un pequeño cartel de confirmación.
---
### 💻 Reproductor General:

&nbsp;El reproductor de musica sera general para ambos tipos de playlist, se encargara de controlar la reproducción de canciones:
* Pausa y Despausa de canciones.
* Saltear a la siguiente y anterior canción (Depende del flujo de canciones).
* Controlar el momento de la canción mediante un slider.
* Opcion para agregar canción a playlist.
* Controlar el flujo de reproducción:
    * **Normal:** Sigue el flujo normal de canciones.
    * **Random:** Se reproduce una canción aleatoria del flujo normal de canciones.
    * **En Ciclo:** Se escucha esta canción de manera repetitiva.

#### 🔗 Flujo de canciones:

&nbsp;Ya que puedes escuchar de manera alternada canciones de Spotify y locales, se debe tener en cuenta un flujo especial de canciones.

* **Escuchando una Playlist:**
    &nbsp;Si se inicia la reproducción desde una playlist ya sea desde el botón de reproducción de la playlist, o de una canción de manera interna a ella desde la pantalla de playlist. Se debe reproducir de manera ordenada desde esa canción reproducida en adelante.
    * **Flujo Random:** Si se tiene activado el flujo random, se debe generar una lista enlacada aleatoria de canciones contenidas en la playlist para asegurarse de que toda canción reproducida de aquí en adelante sea diferente a las anteriores. 
    * **Flujo Normal:** En este caso simplemente se sigue con la reproducción ordenada de la playlist.
* **Escuchando una canción simple:** 
    &nbsp;Si se reproduce una canción desde la página local/ raiz de canciones o desde la pagina de Spotify mediante la barra buscadora solamente se reproducira esa canción y la reproducción se pausara, en caso de que no se haya puesto el **flujo en ciclo** o se hayan puesto canciones **en cola**.
* **Cola de reproducción:**
    &nbsp;La cola de reproducción tiene la prioridad mas alta de reproducción, si se encuentra reproduciendo una playlist pero hay una canción externa a ella (o no) en cola de reproducción se tiene que dar prioridad a las que se encuentran en cola.